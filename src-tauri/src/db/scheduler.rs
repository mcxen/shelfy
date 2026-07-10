use chrono::{DateTime, Utc};
use rusqlite::{params, Result as SqliteResult};
use serde::{Deserialize, Serialize};

use crate::db::get_db;

// ---------------------------------------------------------------------------
// SchedulerLog
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerLog {
    pub id: Option<i64>,
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub event: String,
    pub message: String,
    pub details: Option<String>,
}

pub fn log_scheduler_event(
    level: &str,
    event: &str,
    message: &str,
    details: Option<String>,
) -> SqliteResult<i64> {
    let db = get_db();
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT INTO scheduler_logs (timestamp, level, event, message, details) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![Utc::now().to_rfc3339(), level, event, message, details],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_scheduler_logs(limit: i64) -> SqliteResult<Vec<SchedulerLog>> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, level, event, message, details FROM scheduler_logs ORDER BY timestamp DESC LIMIT ?1",
    )?;
    let logs = stmt
        .query_map(params![limit], |row| {
            let ts_str: String = row.get(1)?;
            Ok(SchedulerLog {
                id: row.get(0)?,
                timestamp: DateTime::parse_from_rfc3339(&ts_str)
                    .unwrap()
                    .with_timezone(&Utc),
                level: row.get(2)?,
                event: row.get(3)?,
                message: row.get(4)?,
                details: row.get(5)?,
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()?;
    Ok(logs)
}

pub fn clear_scheduler_logs() -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM scheduler_logs", [])?;
    Ok(())
}
