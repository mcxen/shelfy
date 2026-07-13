use chrono::{DateTime, Utc};
use rusqlite::{params, Result as SqliteResult};
use serde::{Deserialize, Serialize};

use crate::db::get_db;

// ---------------------------------------------------------------------------
// Orden data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrdenConfigRecord {
    pub id: Option<i64>,
    pub name: String,
    pub yaml: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrdenRunLog {
    pub id: Option<i64>,
    pub config_name: String,
    pub timestamp: DateTime<Utc>,
    pub simulate: bool,
    pub success: i64,
    pub errors: i64,
    pub trigger: String,
    pub logs_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrdenJob {
    pub id: Option<i64>,
    pub name: String,
    pub config_name: String,
    pub enabled: bool,
    pub mode: String,
    pub cron_expr: Option<String>,
    pub fixed_time: Option<String>,
    pub interval_minutes: i64,
    pub watch_paths: String,
    pub tags: String,
    pub skip_tags: String,
    pub simulate: bool,
    pub min_file_count: i64,
    pub path_exists: Option<String>,
    pub time_window_start: Option<String>,
    pub time_window_end: Option<String>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn parse_utc_ts(s: String) -> SqliteResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })
}

fn parse_utc_dt(value: String) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&value)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn row_to_orden_job(row: &rusqlite::Row<'_>) -> SqliteResult<OrdenJob> {
    let last_run_at: Option<String> = row.get(16)?;
    Ok(OrdenJob {
        id: row.get(0)?,
        name: row.get(1)?,
        config_name: row.get(2)?,
        enabled: row.get::<_, i32>(3)? != 0,
        mode: row.get(4)?,
        cron_expr: row.get(5)?,
        fixed_time: row.get(6)?,
        interval_minutes: row.get(7)?,
        watch_paths: row.get(8)?,
        tags: row.get(9)?,
        skip_tags: row.get(10)?,
        simulate: row.get::<_, i32>(11)? != 0,
        min_file_count: row.get(12)?,
        path_exists: row.get(13)?,
        time_window_start: row.get(14)?,
        time_window_end: row.get(15)?,
        last_run_at: last_run_at.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        }),
        created_at: parse_utc_dt(row.get::<_, String>(17)?),
        updated_at: parse_utc_dt(row.get::<_, String>(18)?),
    })
}

// ---------------------------------------------------------------------------
// Orden configs
// ---------------------------------------------------------------------------

pub fn list_orden_config_names() -> SqliteResult<Vec<String>> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare("SELECT name FROM orden_configs ORDER BY name ASC")?;
    let rows: SqliteResult<Vec<String>> = stmt.query_map([], |row| row.get(0))?.collect();
    rows
}

pub fn list_orden_configs() -> SqliteResult<Vec<OrdenConfigRecord>> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, name, yaml, created_at, updated_at FROM orden_configs ORDER BY name ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(OrdenConfigRecord {
            id: row.get(0)?,
            name: row.get(1)?,
            yaml: row.get(2)?,
            created_at: parse_utc_ts(row.get(3)?)?,
            updated_at: parse_utc_ts(row.get(4)?)?,
        })
    })?;
    rows.collect()
}

pub fn get_orden_config(name: &str) -> SqliteResult<Option<OrdenConfigRecord>> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, name, yaml, created_at, updated_at FROM orden_configs WHERE name=?1 LIMIT 1",
    )?;
    let mut rows = stmt.query(params![name])?;
    if let Some(row) = rows.next()? {
        Ok(Some(OrdenConfigRecord {
            id: row.get(0)?,
            name: row.get(1)?,
            yaml: row.get(2)?,
            created_at: parse_utc_ts(row.get(3)?)?,
            updated_at: parse_utc_ts(row.get(4)?)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn upsert_orden_config(name: &str, yaml: &str) -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO orden_configs (name, yaml, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?3)
         ON CONFLICT(name) DO UPDATE SET yaml=excluded.yaml, updated_at=excluded.updated_at",
        params![name, yaml, now],
    )?;
    Ok(())
}

pub fn rename_orden_config(old_name: &str, new_name: &str, yaml: &str) -> SqliteResult<()> {
    let db = get_db();
    let mut conn = db.lock().unwrap();
    let transaction = conn.transaction()?;
    let now = Utc::now().to_rfc3339();
    transaction.execute(
        "UPDATE orden_configs SET name=?1, yaml=?2, updated_at=?3 WHERE name=?4",
        params![new_name, yaml, now, old_name],
    )?;
    transaction.execute(
        "UPDATE orden_jobs SET config_name=?1, updated_at=?2 WHERE config_name=?3",
        params![new_name, now, old_name],
    )?;
    transaction.execute(
        "UPDATE orden_run_logs SET config_name=?1 WHERE config_name=?2",
        params![new_name, old_name],
    )?;
    transaction.commit()
}

pub fn delete_orden_config(name: &str) -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM orden_configs WHERE name=?1", params![name])?;
    conn.execute("DELETE FROM orden_jobs WHERE config_name=?1", params![name])?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Orden jobs
// ---------------------------------------------------------------------------

pub fn list_orden_jobs() -> SqliteResult<Vec<OrdenJob>> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, name, config_name, enabled, mode, cron_expr, fixed_time, interval_minutes, watch_paths, tags, skip_tags, simulate, min_file_count, path_exists, time_window_start, time_window_end, last_run_at, created_at, updated_at FROM orden_jobs ORDER BY name ASC",
    )?;
    let rows = stmt.query_map([], row_to_orden_job)?.collect();
    rows
}

pub fn upsert_orden_job(job: &OrdenJob) -> SqliteResult<i64> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO orden_jobs (name, config_name, enabled, mode, cron_expr, fixed_time, interval_minutes, watch_paths, tags, skip_tags, simulate, min_file_count, path_exists, time_window_start, time_window_end, last_run_at, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?17)
         ON CONFLICT(name) DO UPDATE SET config_name=excluded.config_name, enabled=excluded.enabled, mode=excluded.mode, cron_expr=excluded.cron_expr, fixed_time=excluded.fixed_time, interval_minutes=excluded.interval_minutes, watch_paths=excluded.watch_paths, tags=excluded.tags, skip_tags=excluded.skip_tags, simulate=excluded.simulate, min_file_count=excluded.min_file_count, path_exists=excluded.path_exists, time_window_start=excluded.time_window_start, time_window_end=excluded.time_window_end, updated_at=excluded.updated_at",
        params![
            job.name,
            job.config_name,
            job.enabled as i32,
            job.mode,
            job.cron_expr,
            job.fixed_time,
            job.interval_minutes,
            job.watch_paths,
            job.tags,
            job.skip_tags,
            job.simulate as i32,
            job.min_file_count,
            job.path_exists,
            job.time_window_start,
            job.time_window_end,
            job.last_run_at.map(|dt| dt.to_rfc3339()),
            now,
        ],
    )?;
    conn.query_row(
        "SELECT id FROM orden_jobs WHERE name=?1",
        params![job.name],
        |row| row.get(0),
    )
}

pub fn delete_orden_job(id: i64) -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM orden_jobs WHERE id=?1", params![id])?;
    Ok(())
}

pub fn mark_orden_job_run(id: i64) -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();
    conn.execute(
        "UPDATE orden_jobs SET last_run_at=?1, updated_at=?1 WHERE id=?2",
        params![Utc::now().to_rfc3339(), id],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Orden run logs
// ---------------------------------------------------------------------------

pub fn log_orden_run(
    config_name: &str,
    simulate: bool,
    success: i64,
    errors: i64,
    trigger: &str,
    logs_json: &str,
) -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT INTO orden_run_logs (config_name, timestamp, simulate, success, errors, trigger, logs_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            config_name,
            Utc::now().to_rfc3339(),
            simulate as i32,
            success,
            errors,
            trigger,
            logs_json,
        ],
    )?;
    Ok(())
}

pub fn get_orden_run_logs(config_name: &str, limit: i64) -> SqliteResult<Vec<OrdenRunLog>> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, config_name, timestamp, simulate, success, errors, trigger, logs_json
         FROM orden_run_logs
         WHERE config_name=?1
         ORDER BY timestamp DESC
         LIMIT ?2",
    )?;
    let rows: SqliteResult<Vec<OrdenRunLog>> = stmt
        .query_map(params![config_name, limit], |row| {
            Ok(OrdenRunLog {
                id: row.get(0)?,
                config_name: row.get(1)?,
                timestamp: parse_utc_ts(row.get(2)?)?,
                simulate: row.get::<_, i32>(3)? != 0,
                success: row.get(4)?,
                errors: row.get(5)?,
                trigger: row.get(6)?,
                logs_json: row.get(7)?,
            })
        })?
        .collect();
    rows
}

pub fn get_recent_orden_run_logs(limit: i64) -> SqliteResult<Vec<OrdenRunLog>> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, config_name, timestamp, simulate, success, errors, trigger, logs_json
         FROM orden_run_logs
         ORDER BY timestamp DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |row| {
        Ok(OrdenRunLog {
            id: row.get(0)?,
            config_name: row.get(1)?,
            timestamp: parse_utc_ts(row.get(2)?)?,
            simulate: row.get::<_, i32>(3)? != 0,
            success: row.get(4)?,
            errors: row.get(5)?,
            trigger: row.get(6)?,
            logs_json: row.get(7)?,
        })
    })?;
    rows.collect()
}

pub fn delete_orden_run_log(id: i64) -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM orden_run_logs WHERE id=?1", params![id])?;
    Ok(())
}

pub fn clear_orden_run_logs(config_name: Option<&str>) -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();
    if let Some(config_name) = config_name {
        conn.execute(
            "DELETE FROM orden_run_logs WHERE config_name=?1",
            params![config_name],
        )?;
    } else {
        conn.execute("DELETE FROM orden_run_logs", [])?;
    }
    Ok(())
}
