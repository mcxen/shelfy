use rusqlite::{params, Result as SqliteResult};
use serde::{Deserialize, Serialize};

use crate::db::get_db;

// ---------------------------------------------------------------------------
// Folder modes
// ---------------------------------------------------------------------------

pub const FOLDER_MODE_SILENT: &str = "silent";
pub const FOLDER_MODE_MANUAL: &str = "manual";
pub const FOLDER_MODE_PAUSED: &str = "paused";

pub const FOLDER_MODES: &[&str] = &[FOLDER_MODE_SILENT, FOLDER_MODE_MANUAL, FOLDER_MODE_PAUSED];

pub fn is_folder_auto_mode(mode: &str) -> bool {
    mode == FOLDER_MODE_SILENT
}

pub fn is_folder_manual_mode(mode: &str) -> bool {
    mode == FOLDER_MODE_MANUAL
}

pub fn is_folder_paused_mode(mode: &str) -> bool {
    mode == FOLDER_MODE_PAUSED
}

pub fn is_valid_folder_mode(mode: &str) -> bool {
    FOLDER_MODES.contains(&mode)
}

// ---------------------------------------------------------------------------
// WatchedFolder
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchedFolder {
    pub id: Option<i64>,
    pub path: String,
    pub enabled: bool,
    /// One of: "silent" (real-time auto-organize), "manual" (collect only),
    /// "paused" (do not watch).
    pub mode: String,
}

pub fn get_watched_folders() -> SqliteResult<Vec<WatchedFolder>> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id, path, enabled, mode FROM watched_folders")?;
    let folders = stmt
        .query_map([], |row| {
            Ok(WatchedFolder {
                id: row.get(0)?,
                path: row.get(1)?,
                enabled: row.get::<_, i32>(2)? != 0,
                mode: row.get(3)?,
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()?;
    Ok(folders)
}

pub fn add_watched_folder(path: &str, mode: &str) -> SqliteResult<i64> {
    let db = get_db();
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO watched_folders (path, enabled, mode) VALUES (?1, 1, ?2)",
        params![path, mode],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn add_watched_folder_record(folder: &WatchedFolder, preserve_id: bool) -> SqliteResult<i64> {
    let db = get_db();
    let conn = db.lock().unwrap();
    if preserve_id {
        conn.execute(
            "INSERT OR REPLACE INTO watched_folders (id, path, enabled, mode) VALUES (?1, ?2, ?3, ?4)",
            params![folder.id, folder.path, folder.enabled as i32, folder.mode],
        )?;
    } else {
        conn.execute(
            "INSERT OR IGNORE INTO watched_folders (path, enabled, mode) VALUES (?1, ?2, ?3)",
            params![folder.path, folder.enabled as i32, folder.mode],
        )?;
    }
    Ok(conn.last_insert_rowid())
}

pub fn remove_watched_folder(id: i64) -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM watched_folders WHERE id=?1", params![id])?;
    Ok(())
}

pub fn delete_all_watched_folders() -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM watched_folders", [])?;
    Ok(())
}

pub fn update_folder_mode(id: i64, mode: &str) -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();
    conn.execute(
        "UPDATE watched_folders SET mode=?1 WHERE id=?2",
        params![mode, id],
    )?;
    Ok(())
}
