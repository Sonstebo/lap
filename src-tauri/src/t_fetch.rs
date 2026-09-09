/**
 * t_fetch.rs - Fetch on open
 *
 * An album may name a command that materialises a file which is not on disk
 * when it is opened: a cold-storage tier, a NAS that sleeps, a cloud photo
 * cache that keeps only thumbnails until a preview is asked for. The command
 * is stored per album (`albums.fetch_command`) and runs through `sh -c` with
 * `{path}` replaced by the missing file's path and `{name}` by its file name,
 * both shell-quoted. One fetch per path runs at a time; a request that arrives
 * while one is in flight waits for it. The request continues only if the file
 * exists afterwards, so a command that fails or produces nothing leaves the
 * previous "not found" behaviour untouched.
 */
use std::collections::HashSet;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// How long one fetch may take before it is killed. Generous: the file may be a
/// full-resolution photo or a video coming over a slow link.
const FETCH_TIMEOUT: Duration = Duration::from_secs(900);

static IN_FLIGHT: OnceLock<(Mutex<HashSet<String>>, Condvar)> = OnceLock::new();

fn in_flight() -> &'static (Mutex<HashSet<String>>, Condvar) {
    IN_FLIGHT.get_or_init(|| (Mutex::new(HashSet::new()), Condvar::new()))
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Render the album's command for one path.
pub fn render(template: &str, file_path: &str) -> String {
    let name = Path::new(file_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    template
        .replace("{path}", &shell_quote(file_path))
        .replace("{name}", &shell_quote(&name))
}

/// Run `template` for `file_path` if the file is missing; true when the file exists afterwards.
/// Blocking: call from a blocking task.
pub fn ensure_present(template: Option<&str>, file_path: &str) -> bool {
    if Path::new(file_path).exists() {
        return true;
    }
    let Some(template) = template.filter(|t| !t.trim().is_empty()) else {
        return false;
    };
    let (set, cv) = in_flight();
    {
        let mut guard = set.lock().unwrap_or_else(|e| e.into_inner());
        while guard.contains(file_path) {
            guard = cv.wait(guard).unwrap_or_else(|e| e.into_inner());
        }
        if Path::new(file_path).exists() {
            return true; // another request fetched it meanwhile
        }
        guard.insert(file_path.to_string());
    }
    let command = render(template, file_path);
    let outcome = run_with_timeout(&command, FETCH_TIMEOUT);
    {
        let mut guard = set.lock().unwrap_or_else(|e| e.into_inner());
        guard.remove(file_path);
        cv.notify_all();
    }
    let present = Path::new(file_path).exists();
    match outcome {
        Ok(status) if status.success() && present => {}
        Ok(status) => eprintln!("fetch on open: `{}` exited with {} and the file {} present", command, status,
                                if present { "is" } else { "is not" }),
        Err(err) => eprintln!("fetch on open: `{}` failed: {}", command, err),
    }
    present
}

fn run_with_timeout(command: &str, timeout: Duration) -> Result<std::process::ExitStatus, String> {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| e.to_string())?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if started.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("timed out after {} s", timeout.as_secs()));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => return Err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_quoted_placeholders() {
        let cmd = render("fetch {path} {name}", "/tmp/a b/it's.jpg");
        assert_eq!(cmd, "fetch '/tmp/a b/it'\\''s.jpg' 'it'\\''s.jpg'");
    }

    #[test]
    fn present_file_needs_no_command() {
        let dir = std::env::temp_dir();
        let path = dir.join("lap-fetch-present.txt");
        std::fs::write(&path, b"x").unwrap();
        assert!(ensure_present(None, path.to_str().unwrap()));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn command_that_creates_the_file_succeeds_and_one_that_does_not_fails() {
        let dir = std::env::temp_dir();
        let path = dir.join("lap-fetch-made.txt");
        let _ = std::fs::remove_file(&path);
        assert!(!ensure_present(None, path.to_str().unwrap()));
        assert!(ensure_present(Some("echo made > {path}"), path.to_str().unwrap()));
        let _ = std::fs::remove_file(&path);
        assert!(!ensure_present(Some("true"), path.to_str().unwrap()));
    }
}
