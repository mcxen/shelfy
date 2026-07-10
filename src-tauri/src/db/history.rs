use chrono::{DateTime, Utc};
use rusqlite::{params, Result as SqliteResult};
use serde::{Deserialize, Serialize};

use crate::db::{get_db, DB};

// ---------------------------------------------------------------------------
// ActionLog
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionLog {
    pub id: Option<i64>,
    pub timestamp: DateTime<Utc>,
    pub source_path: String,
    pub destination_path: Option<String>,
    pub action: String,
    pub file_name: String,
    pub file_type: String,
    pub engine: String,
    pub rule_label: Option<String>,
    pub undone: bool,
}

pub fn log_action(log: &ActionLog) -> SqliteResult<i64> {
    let db = get_db();
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT INTO action_logs (timestamp, source_path, destination_path, action, file_name, file_type, engine, rule_label, undone) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0)",
        params![
            log.timestamp.to_rfc3339(),
            log.source_path,
            log.destination_path,
            log.action,
            log.file_name,
            log.file_type,
            log.engine,
            log.rule_label
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn log_action_if_initialized(log: &ActionLog) -> SqliteResult<Option<i64>> {
    let Some(db) = DB.get() else {
        return Ok(None);
    };
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT INTO action_logs (timestamp, source_path, destination_path, action, file_name, file_type, engine, rule_label, undone) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0)",
        params![
            log.timestamp.to_rfc3339(),
            log.source_path,
            log.destination_path,
            log.action,
            log.file_name,
            log.file_type,
            log.engine,
            log.rule_label
        ],
    )?;
    Ok(Some(conn.last_insert_rowid()))
}

pub fn get_recent_logs(limit: i64) -> SqliteResult<Vec<ActionLog>> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, source_path, destination_path, action, file_name, file_type, engine, rule_label, undone FROM action_logs ORDER BY timestamp DESC LIMIT ?1"
    )?;
    let logs = stmt
        .query_map(params![limit], |row| {
            let ts_str: String = row.get(1)?;
            Ok(ActionLog {
                id: row.get(0)?,
                timestamp: DateTime::parse_from_rfc3339(&ts_str)
                    .unwrap()
                    .with_timezone(&Utc),
                source_path: row.get(2)?,
                destination_path: row.get(3)?,
                action: row.get(4)?,
                file_name: row.get(5)?,
                file_type: row.get(6)?,
                engine: row.get(7)?,
                rule_label: row.get(8)?,
                undone: row.get::<_, i32>(9)? != 0,
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()?;
    Ok(logs)
}

pub fn get_undoable_logs() -> SqliteResult<Vec<(i64, String, String, String)>> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, source_path, destination_path, action FROM action_logs WHERE undone = 0 ORDER BY timestamp DESC"
    )?;
    let logs = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<SqliteResult<Vec<_>>>()?;
    Ok(logs)
}

pub fn get_weekly_stats() -> SqliteResult<Vec<(String, i64)>> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT file_type, COUNT(*) FROM action_logs WHERE timestamp > datetime('now', '-7 days') AND undone = 0 GROUP BY file_type"
    )?;
    let stats = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?
        .collect::<SqliteResult<Vec<_>>>()?;
    Ok(stats)
}

pub fn undo_action(id: i64) -> SqliteResult<Option<(String, String)>> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let log: Option<(String, String)> = conn
        .query_row(
            "SELECT source_path, destination_path FROM action_logs WHERE id=?1 AND undone=0",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();

    if log.is_some() {
        conn.execute("UPDATE action_logs SET undone=1 WHERE id=?1", params![id])?;
    }
    Ok(log)
}

pub fn clear_logs() -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM action_logs", [])?;
    Ok(())
}

pub fn delete_log(id: i64) -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM action_logs WHERE id=?1", params![id])?;
    Ok(())
}
