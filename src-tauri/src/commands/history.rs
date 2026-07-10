use crate::db::{
    clear_logs, get_db, get_recent_logs, get_undoable_logs, get_weekly_stats, ActionLog,
};
use crate::AppState;
use std::time::Instant;

#[tauri::command]
pub fn get_logs_cmd(limit: i64) -> Result<Vec<ActionLog>, String> {
    get_recent_logs(limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_stats_cmd() -> Result<Vec<(String, i64)>, String> {
    get_weekly_stats().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn undo_action_cmd(id: i64, state: tauri::State<AppState>) -> Result<bool, String> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let log: Option<(String, String, String)> = conn
        .query_row(
            "SELECT source_path, destination_path, action FROM action_logs WHERE id=?1 AND undone=0",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .ok();

    if let Some((source, dest, action)) = log {
        if !dest.is_empty() && std::path::Path::new(&dest).exists() {
            if action == "copy" {
                let _ = std::fs::remove_file(&dest);
            } else {
                let _ = std::fs::rename(&dest, &source);
                // Ignore this file for 5 seconds so the watcher doesn't re-process it
                let mut ignored = state.ignored_files.lock().unwrap();
                ignored.insert(source, Instant::now());
            }
        }
        conn.execute("UPDATE action_logs SET undone=1 WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub fn undo_all_cmd(state: tauri::State<AppState>) -> Result<i32, String> {
    let logs = get_undoable_logs().map_err(|e| e.to_string())?;
    let db = get_db();
    let conn = db.lock().unwrap();
    let mut count = 0;

    for (id, source, dest, action) in logs {
        if !dest.is_empty() && std::path::Path::new(&dest).exists() {
            if action == "copy" {
                let _ = std::fs::remove_file(&dest);
            } else {
                let _ = std::fs::rename(&dest, &source);
                let mut ignored = state.ignored_files.lock().unwrap();
                ignored.insert(source, Instant::now());
            }
        }
        let _ = conn.execute("UPDATE action_logs SET undone=1 WHERE id=?1", [id]);
        count += 1;
    }

    Ok(count)
}

#[tauri::command]
pub fn clear_logs_cmd() -> Result<(), String> {
    clear_logs().map_err(|e| e.to_string())
}
