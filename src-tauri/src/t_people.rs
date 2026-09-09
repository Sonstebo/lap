/**
 * t_people.rs - Naming people, a group at a time
 *
 * Face recognition compares a face against reference faces, which cannot work
 * across a childhood: a five-year-old does not resemble an eighteen-year-old
 * closely enough to pass any threshold that is not also wrong about everyone
 * else. The album's tool solves it by grouping faces that chain together through
 * the ages between, so naming one group names a whole life.
 *
 *   <cli> --json faces cluster              build the groups
 *   <cli> --json faces groups [--unnamed]   what they are
 *   <cli> --json faces crop <id>...         a picture of the face being asked about
 *   <cli> --json faces name <group> <who>   name a whole group
 *   <cli> --json index --rematch            spread the new seeds
 *
 * This window only asks; every decision is the tool's, and the tool is where the
 * names live, so nothing said here is lost the next time the library is exported.
 */
use std::time::Duration;

use serde_json::Value;

use crate::t_ask::run_tool;

/// Grouping thirty thousand faces is arithmetic, not inference, but it is not instant.
const GROUP_TIMEOUT: Duration = Duration::from_secs(1800);
const ASK_TIMEOUT: Duration = Duration::from_secs(300);

fn tool() -> Result<String, String> {
    crate::t_sqlite::AFile::any_tool()
        .ok_or_else(|| "no album names a tool that keeps the names of people".to_string())
}

async fn ask(args: Vec<String>, timeout: Duration) -> Result<Value, String> {
    let cli = tool()?;
    tauri::async_runtime::spawn_blocking(move || run_tool(&cli, &args, timeout))
        .await
        .map_err(|e| e.to_string())?
}

/// Build the groups. Safe to run again: it replaces what was there.
#[tauri::command]
pub async fn people_build_groups(threshold: Option<f64>) -> Result<Value, String> {
    let mut args = vec!["faces".to_string(), "cluster".to_string()];
    if let Some(t) = threshold {
        args.push("--threshold".into());
        args.push(format!("{:.2}", t.clamp(0.3, 0.95)));
    }
    ask(args, GROUP_TIMEOUT).await
}

/// The groups, largest first.
#[tauri::command]
pub async fn people_groups(limit: Option<i64>, unnamed_only: bool,
                           min_size: Option<i64>) -> Result<Value, String> {
    let mut args = vec!["faces".to_string(), "groups".to_string(),
                        "--limit".to_string(), limit.unwrap_or(40).clamp(1, 500).to_string()];
    if unnamed_only {
        args.push("--unnamed".into());
    }
    if let Some(n) = min_size {
        args.push("--min-size".into());
        args.push(n.max(1).to_string());
    }
    ask(args, ASK_TIMEOUT).await
}

/// A picture of each face, so the question has a face attached to it.
#[tauri::command]
pub async fn people_face_crops(face_ids: Vec<i64>, size: Option<i64>) -> Result<Value, String> {
    if face_ids.is_empty() {
        return Ok(Value::Array(vec![]));
    }
    let mut args = vec!["faces".to_string(), "crop".to_string()];
    args.extend(face_ids.iter().take(64).map(|i| i.to_string()));
    args.push("--size".into());
    args.push(size.unwrap_or(160).clamp(48, 512).to_string());
    ask(args, ASK_TIMEOUT).await
}

/// Name a whole group. Faces that already name someone else are left alone.
#[tauri::command]
pub async fn people_name_group(cluster: i64, person: String, force: bool) -> Result<Value, String> {
    if person.trim().is_empty() {
        return Err("say who it is".into());
    }
    let mut args = vec!["faces".to_string(), "name".to_string(),
                        cluster.to_string(), person.trim().to_string()];
    if force {
        args.push("--force".into());
    }
    ask(args, ASK_TIMEOUT).await
}

/// Undo naming a group; a name given by hand is kept.
#[tauri::command]
pub async fn people_unname_group(cluster: i64) -> Result<Value, String> {
    ask(vec!["faces".to_string(), "unname".to_string(), cluster.to_string()], ASK_TIMEOUT).await
}

/// Spread the new seeds over the faces that were never named by hand.
#[tauri::command]
pub async fn people_rematch() -> Result<Value, String> {
    ask(vec!["index".to_string(), "--rematch".to_string()], GROUP_TIMEOUT).await
}
