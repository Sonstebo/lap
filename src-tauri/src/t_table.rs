/**
 * t_table.rs - The light table: many photos into one picture
 *
 * The photo editor asks the album's tool to change one photograph. This asks the
 * same tool to lay out a set of them: a collage, a book spread, a contact strip.
 *
 *   <cli> --json compose <collection> --template T --shape S ...   a draft
 *   <cli> --json compose <collection> ... --originals              at print size
 *   <cli> --json select <brief> --into <collection>                fill a set
 *   <cli> --json collection list|show                              the sets there are
 *
 * The draft is the renderer, not an approximation of it: the same code draws the
 * page at screen size in a fraction of a second and at 300 dpi when asked, so what
 * is approved on screen is what comes out of the printer. Drafts alternate between
 * two files so the webview always sees a new address and never a stale picture.
 */
use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

use crate::t_ask::run_tool;

/// A draft is local work on cached thumbnails; it should be quick.
const DRAFT_TIMEOUT: Duration = Duration::from_secs(180);
/// Printing may fetch full-resolution originals over the network first.
const RENDER_TIMEOUT: Duration = Duration::from_secs(1800);

fn tool() -> Result<String, String> {
    crate::t_sqlite::AFile::any_tool()
        .ok_or_else(|| "no album names a tool that can compose its photos".to_string())
}

/// Where drafts live: under the user's home, so the asset protocol can serve them.
fn draft_path(slot: u8) -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "no HOME".to_string())?;
    let dir = PathBuf::from(home).join(".cache").join("photos-light-table");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(format!("draft-{}.jpg", slot % 2)))
}

/// The arguments shared by a draft and a print.
fn compose_args(collection: Option<String>, paths: Option<Vec<String>>, template: &str,
                shape: &str, gap: i64, count: Option<i64>, face_safe: bool) -> Result<Vec<String>, String> {
    let mut args = vec!["compose".to_string()];
    match (&collection, &paths) {
        (Some(name), _) if !name.trim().is_empty() => args.push(name.clone()),
        (_, Some(list)) if !list.is_empty() => {
            args.push("--id".into());
            args.extend(list.iter().cloned());
        }
        _ => return Err("nothing to compose: name a collection or select some photos".into()),
    }
    args.push("--template".into());
    args.push(template.to_string());
    args.push("--shape".into());
    args.push(shape.to_string());
    args.push("--gap".into());
    args.push(gap.to_string());
    if let Some(n) = count.filter(|n| *n > 0) {
        args.push("--count".into());
        args.push(n.to_string());
    }
    if !face_safe {
        args.push("--no-face-safe".into());
    }
    Ok(args)
}

/// Lay the set out at screen size, from what is already cached. No network.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn light_table_draft(collection: Option<String>, paths: Option<Vec<String>>,
                               template: String, shape: String, gap: i64, count: Option<i64>,
                               face_safe: bool, slot: u8, long_edge: Option<i64>) -> Result<Value, String> {
    let cli = tool()?;
    let out = draft_path(slot)?;
    let mut args = compose_args(collection, paths, &template, &shape, gap, count, face_safe)?;
    args.push("--long-edge".into());
    args.push(long_edge.filter(|n| *n > 0).unwrap_or(1400).to_string());
    args.push("--out".into());
    args.push(out.to_string_lossy().to_string());
    tauri::async_runtime::spawn_blocking(move || run_tool(&cli, &args, DRAFT_TIMEOUT))
        .await
        .map_err(|e| e.to_string())?
}

/// The same layout at print size, fetching originals. This one costs bytes and time.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn light_table_render(collection: Option<String>, paths: Option<Vec<String>>,
                                template: String, shape: String, gap: i64, count: Option<i64>,
                                face_safe: bool) -> Result<Value, String> {
    let cli = tool()?;
    let mut args = compose_args(collection, paths, &template, &shape, gap, count, face_safe)?;
    args.push("--originals".into());
    tauri::async_runtime::spawn_blocking(move || run_tool(&cli, &args, RENDER_TIMEOUT))
        .await
        .map_err(|e| e.to_string())?
}

/// The sets that exist, with how many photos each holds.
#[tauri::command]
pub async fn light_table_collections() -> Result<Value, String> {
    let cli = tool()?;
    let args = vec!["collection".to_string(), "list".to_string()];
    tauri::async_runtime::spawn_blocking(move || run_tool(&cli, &args, DRAFT_TIMEOUT))
        .await
        .map_err(|e| e.to_string())?
}

/// Ask the library to choose a set and put it in a collection.
#[tauri::command]
pub async fn light_table_select(brief: Option<String>, count: i64, variety: f64,
                                spread: String, into: String) -> Result<Value, String> {
    let cli = tool()?;
    if into.trim().is_empty() {
        return Err("name the collection to put them in".into());
    }
    let mut args = vec!["select".to_string()];
    if let Some(text) = brief.as_ref().filter(|t| !t.trim().is_empty()) {
        args.push(text.clone());
    }
    args.push("--count".into());
    args.push(count.max(1).to_string());
    args.push("--variety".into());
    args.push(format!("{:.2}", variety.clamp(0.0, 1.0)));
    if spread != "none" {
        args.push("--spread".into());
        args.push(spread);
    }
    args.push("--into".into());
    args.push(into);
    args.push("--replace".into());
    tauri::async_runtime::spawn_blocking(move || run_tool(&cli, &args, RENDER_TIMEOUT))
        .await
        .map_err(|e| e.to_string())?
}

/// The photos in one of this library's collections, as paths the tool understands.
#[tauri::command]
pub fn light_table_collection_paths(collection_id: i64) -> Vec<String> {
    crate::t_sqlite::AFile::paths_in_collection(collection_id)
}

#[cfg(test)]
mod tests {
    use super::compose_args;

    #[test]
    fn a_collection_wins_over_a_selection_and_flags_follow() {
        let args = compose_args(Some("Winter".into()), Some(vec!["/a.jpg".into()]),
                                "grid", "square", 12, Some(9), false).unwrap();
        assert_eq!(args[0], "compose");
        assert_eq!(args[1], "Winter");
        assert!(args.contains(&"--no-face-safe".to_string()));
        assert!(args.windows(2).any(|w| w[0] == "--count" && w[1] == "9"));
        assert!(!args.contains(&"--id".to_string()));
    }

    #[test]
    fn selected_photos_are_passed_as_ids() {
        let args = compose_args(None, Some(vec!["/a.jpg".into(), "/b.jpg".into()]),
                                "justified", "3:2", 8, None, true).unwrap();
        assert!(args.contains(&"--id".to_string()));
        assert!(args.contains(&"/b.jpg".to_string()));
        assert!(!args.contains(&"--no-face-safe".to_string()));
        assert!(!args.contains(&"--count".to_string()));
    }

    #[test]
    fn nothing_to_compose_is_refused_before_the_tool_runs() {
        assert!(compose_args(None, None, "grid", "3:2", 8, None, true).is_err());
        assert!(compose_args(Some("  ".into()), Some(vec![]), "grid", "3:2", 8, None, true).is_err());
    }
}
