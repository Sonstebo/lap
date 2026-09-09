/**
 * t_ask.rs - Asking the album's tool to edit a photo
 *
 * A managed album may name the tool that maintains it (`albums.cli`). That tool
 * is a program, not a shell line: it is run with an argument list, so nothing a
 * user types can turn into a command. Three things are asked of it, all with
 * `--json`:
 *
 *   <cli> --json edit <file> <prompt> [--from N] --background   queue an edit
 *   <cli> --json versions <file>                                the photo's versions
 *   <cli> --json jobs --limit N                                 what is running
 *
 * The edit runs in the background because a generated one takes a minute or
 * two; the interface polls `jobs` and shows the version when it appears.
 */
use std::process::Command;
use std::time::Duration;

use serde_json::Value;

/// How long the tool may take to answer. These calls only queue or report.
const CALL_TIMEOUT: Duration = Duration::from_secs(120);

fn run(cli: &str, args: &[String]) -> Result<Value, String> {
    let mut command = Command::new(cli);
    command.arg("--json").args(args);
    let child = command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("{cli}: {e}"))?;
    let out = wait_with_timeout(child, CALL_TIMEOUT)?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    if !out.status.success() {
        // The tool's contract: one line of JSON on stderr with a code and a message.
        let stderr = String::from_utf8_lossy(&out.stderr);
        let message = stderr
            .lines()
            .rev()
            .find_map(|line| serde_json::from_str::<Value>(line).ok())
            .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(str::to_string))
            .unwrap_or_else(|| stderr.lines().last().unwrap_or("the tool failed").to_string());
        return Err(message);
    }
    serde_json::from_str(stdout.trim()).map_err(|e| format!("unreadable answer from {cli}: {e}"))
}

fn wait_with_timeout(mut child: std::process::Child, timeout: Duration) -> Result<std::process::Output, String> {
    let started = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().map_err(|e| e.to_string()),
            Ok(None) if started.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("the tool did not answer within {} s", timeout.as_secs()));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => return Err(e.to_string()),
        }
    }
}

/// Queue an edit of `file_id`; returns the job.
#[tauri::command]
pub async fn photo_edit_start(file_id: i64, prompt: String, from_version: Option<i64>) -> Result<Value, String> {
    let (cli, path) = crate::t_sqlite::AFile::tool_for(file_id)
        .ok_or("this album has no tool that can edit its photos")?;
    if prompt.trim().is_empty() {
        return Err("say what to change".into());
    }
    let mut args = vec!["edit".to_string(), path, prompt, "--background".to_string()];
    if let Some(from) = from_version {
        args.push("--from".into());
        args.push(from.to_string());
    }
    tauri::async_runtime::spawn_blocking(move || run(&cli, &args))
        .await
        .map_err(|e| e.to_string())?
}

/// The versions made from `file_id`, oldest first.
#[tauri::command]
pub async fn photo_versions(file_id: i64) -> Result<Value, String> {
    let (cli, path) = crate::t_sqlite::AFile::tool_for(file_id)
        .ok_or("this album has no tool that keeps versions")?;
    let args = vec!["versions".to_string(), path];
    tauri::async_runtime::spawn_blocking(move || run(&cli, &args))
        .await
        .map_err(|e| e.to_string())?
}

/// Queued, running and finished edits across the library.
#[tauri::command]
pub async fn photo_edit_jobs(limit: Option<i64>) -> Result<Value, String> {
    let cli = crate::t_sqlite::AFile::any_tool().ok_or("no album names a tool")?;
    let args = vec!["jobs".to_string(), "--limit".to_string(), limit.unwrap_or(20).to_string()];
    tauri::async_runtime::spawn_blocking(move || run(&cli, &args))
        .await
        .map_err(|e| e.to_string())?
}

/// Whether this file belongs to an album with a tool, so the interface can show the option.
#[tauri::command]
pub fn photo_can_edit(file_id: i64) -> bool {
    crate::t_sqlite::AFile::tool_for(file_id).is_some()
}
