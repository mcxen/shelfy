use chrono::{DateTime, Utc};
use once_cell::sync::OnceCell;
use rusqlite::{params, Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: Option<i64>,
    pub name: String,
    pub priority: i32,
    pub enabled: bool,
    pub extensions: Vec<String>,
    pub pattern: Option<String>,
    pub destination: String,
    pub action: String, // "move", "rename", "delete", "ignore"
    pub folder_id: i64,
    #[serde(default)]
    pub folder_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchedFolder {
    pub id: Option<i64>,
    pub path: String,
    pub enabled: bool,
    /// One of: "silent" (real-time auto-organize), "manual" (collect only),
    /// "paused" (do not watch).
    pub mode: String,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub id: Option<i64>,
    pub language: String,
    pub theme: String,
    pub telemetry_enabled: bool,
    pub first_run: bool,
    pub autostart: bool,
    pub grace_period_seconds: i64,
    pub lock_check_enabled: bool,
    pub schedule_enabled: bool,
    pub schedule_times_per_day: i64,
    pub schedule_time_1: Option<String>,
    pub schedule_time_2: Option<String>,
    pub schedule_time_3: Option<String>,
    pub schedule_time_4: Option<String>,
    pub schedule_cron_enabled: bool,
    pub schedule_cron_expr: Option<String>,
    pub keepalive_enabled: bool,
    pub keepalive_interval_minutes: i64,
    #[serde(default)]
    pub mcp_enabled: bool,
    #[serde(default)]
    pub mcp_allow_write: bool,
    #[serde(default = "default_mcp_transport")]
    pub mcp_transport: String,
    #[serde(default = "default_mcp_server_name")]
    pub mcp_server_name: String,
    #[serde(default)]
    pub mcp_command: Option<String>,
    #[serde(default)]
    pub mcp_args: Option<String>,
    #[serde(default)]
    pub mcp_http_url: Option<String>,
    #[serde(default)]
    pub mcp_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleSettings {
    pub schedule_enabled: bool,
    pub schedule_times_per_day: i64,
    pub schedule_time_1: Option<String>,
    pub schedule_time_2: Option<String>,
    pub schedule_time_3: Option<String>,
    pub schedule_time_4: Option<String>,
    pub schedule_cron_enabled: bool,
    pub schedule_cron_expr: Option<String>,
    pub keepalive_enabled: bool,
    pub keepalive_interval_minutes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerLog {
    pub id: Option<i64>,
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub event: String,
    pub message: String,
    pub details: Option<String>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSnapshot {
    pub settings: AppSettings,
    pub folders: Vec<WatchedFolder>,
    pub rules: Vec<Rule>,
}

fn default_mcp_transport() -> String {
    "stdio".to_string()
}

fn default_mcp_server_name() -> String {
    "shelfy".to_string()
}

static DB: OnceCell<Arc<Mutex<Connection>>> = OnceCell::new();

pub fn init_db(app_dir: PathBuf) -> SqliteResult<()> {
    let db_path = app_dir.join("shelfy.db");
    let conn = Connection::open(db_path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS watched_folders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL UNIQUE,
            enabled INTEGER NOT NULL DEFAULT 1,
            mode TEXT NOT NULL DEFAULT 'silent'
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS rules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            priority INTEGER NOT NULL DEFAULT 0,
            enabled INTEGER NOT NULL DEFAULT 1,
            extensions TEXT NOT NULL,
            pattern TEXT,
            destination TEXT NOT NULL,
            action TEXT NOT NULL DEFAULT 'move',
            folder_id INTEGER NOT NULL DEFAULT 0,
            folder_path TEXT
        )",
        [],
    )?;

    let rule_cols: Vec<String> = conn
        .prepare("PRAGMA table_info(rules)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    if !rule_cols.iter().any(|c| c == "folder_path") {
        conn.execute("ALTER TABLE rules ADD COLUMN folder_path TEXT", [])?;
    }

    conn.execute(
        "CREATE TABLE IF NOT EXISTS action_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            source_path TEXT NOT NULL,
            destination_path TEXT,
            action TEXT NOT NULL,
            file_name TEXT NOT NULL,
            file_type TEXT NOT NULL,
            engine TEXT NOT NULL DEFAULT 'rules',
            rule_label TEXT,
            undone INTEGER NOT NULL DEFAULT 0
        )",
        [],
    )?;

    let action_log_cols: Vec<String> = conn
        .prepare("PRAGMA table_info(action_logs)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    if !action_log_cols.iter().any(|c| c == "engine") {
        conn.execute(
            "ALTER TABLE action_logs ADD COLUMN engine TEXT NOT NULL DEFAULT 'rules'",
            [],
        )?;
    }
    if !action_log_cols.iter().any(|c| c == "rule_label") {
        conn.execute("ALTER TABLE action_logs ADD COLUMN rule_label TEXT", [])?;
    }

    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            language TEXT NOT NULL DEFAULT 'en',
            theme TEXT NOT NULL DEFAULT 'system',
            telemetry_enabled INTEGER NOT NULL DEFAULT 0,
            first_run INTEGER NOT NULL DEFAULT 1,
            autostart INTEGER NOT NULL DEFAULT 1
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS scheduler_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            level TEXT NOT NULL,
            event TEXT NOT NULL,
            message TEXT NOT NULL,
            details TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS orden_configs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            yaml TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS orden_run_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            config_name TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            simulate INTEGER NOT NULL DEFAULT 0,
            success INTEGER NOT NULL DEFAULT 0,
            errors INTEGER NOT NULL DEFAULT 0,
            trigger TEXT NOT NULL DEFAULT 'manual',
            logs_json TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS orden_jobs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            config_name TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            mode TEXT NOT NULL DEFAULT 'manual',
            cron_expr TEXT,
            fixed_time TEXT,
            interval_minutes INTEGER NOT NULL DEFAULT 60,
            watch_paths TEXT NOT NULL DEFAULT '',
            tags TEXT NOT NULL DEFAULT '',
            skip_tags TEXT NOT NULL DEFAULT '',
            simulate INTEGER NOT NULL DEFAULT 0,
            min_file_count INTEGER NOT NULL DEFAULT 0,
            path_exists TEXT,
            time_window_start TEXT,
            time_window_end TEXT,
            last_run_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;

    // Migration: add missing columns
    let cols: Vec<String> = conn
        .prepare("PRAGMA table_info(settings)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    if !cols.iter().any(|c| c == "autostart") {
        conn.execute(
            "ALTER TABLE settings ADD COLUMN autostart INTEGER NOT NULL DEFAULT 1",
            [],
        )?;
    }
    if !cols.iter().any(|c| c == "grace_period_seconds") {
        conn.execute(
            "ALTER TABLE settings ADD COLUMN grace_period_seconds INTEGER NOT NULL DEFAULT 300",
            [],
        )?;
    }
    if !cols.iter().any(|c| c == "lock_check_enabled") {
        conn.execute(
            "ALTER TABLE settings ADD COLUMN lock_check_enabled INTEGER NOT NULL DEFAULT 1",
            [],
        )?;
    }
    if !cols.iter().any(|c| c == "schedule_enabled") {
        conn.execute(
            "ALTER TABLE settings ADD COLUMN schedule_enabled INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !cols.iter().any(|c| c == "schedule_times_per_day") {
        conn.execute(
            "ALTER TABLE settings ADD COLUMN schedule_times_per_day INTEGER NOT NULL DEFAULT 1",
            [],
        )?;
    }
    if !cols.iter().any(|c| c == "schedule_time_1") {
        conn.execute("ALTER TABLE settings ADD COLUMN schedule_time_1 TEXT", [])?;
    }
    if !cols.iter().any(|c| c == "schedule_time_2") {
        conn.execute("ALTER TABLE settings ADD COLUMN schedule_time_2 TEXT", [])?;
    }
    if !cols.iter().any(|c| c == "schedule_time_3") {
        conn.execute("ALTER TABLE settings ADD COLUMN schedule_time_3 TEXT", [])?;
    }
    if !cols.iter().any(|c| c == "schedule_time_4") {
        conn.execute("ALTER TABLE settings ADD COLUMN schedule_time_4 TEXT", [])?;
    }
    if !cols.iter().any(|c| c == "schedule_cron_enabled") {
        conn.execute(
            "ALTER TABLE settings ADD COLUMN schedule_cron_enabled INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !cols.iter().any(|c| c == "schedule_cron_expr") {
        conn.execute(
            "ALTER TABLE settings ADD COLUMN schedule_cron_expr TEXT",
            [],
        )?;
    }
    if !cols.iter().any(|c| c == "keepalive_enabled") {
        conn.execute(
            "ALTER TABLE settings ADD COLUMN keepalive_enabled INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !cols.iter().any(|c| c == "keepalive_interval_minutes") {
        conn.execute(
            "ALTER TABLE settings ADD COLUMN keepalive_interval_minutes INTEGER NOT NULL DEFAULT 15",
            [],
        )?;
    }
    if !cols.iter().any(|c| c == "mcp_enabled") {
        conn.execute(
            "ALTER TABLE settings ADD COLUMN mcp_enabled INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !cols.iter().any(|c| c == "mcp_allow_write") {
        conn.execute(
            "ALTER TABLE settings ADD COLUMN mcp_allow_write INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !cols.iter().any(|c| c == "mcp_transport") {
        conn.execute(
            "ALTER TABLE settings ADD COLUMN mcp_transport TEXT NOT NULL DEFAULT 'stdio'",
            [],
        )?;
    }
    if !cols.iter().any(|c| c == "mcp_server_name") {
        conn.execute(
            "ALTER TABLE settings ADD COLUMN mcp_server_name TEXT NOT NULL DEFAULT 'shelfy'",
            [],
        )?;
    }
    if !cols.iter().any(|c| c == "mcp_command") {
        conn.execute("ALTER TABLE settings ADD COLUMN mcp_command TEXT", [])?;
    }
    if !cols.iter().any(|c| c == "mcp_args") {
        conn.execute("ALTER TABLE settings ADD COLUMN mcp_args TEXT", [])?;
    }
    if !cols.iter().any(|c| c == "mcp_http_url") {
        conn.execute("ALTER TABLE settings ADD COLUMN mcp_http_url TEXT", [])?;
    }
    if !cols.iter().any(|c| c == "mcp_token") {
        conn.execute("ALTER TABLE settings ADD COLUMN mcp_token TEXT", [])?;
    }
    // Insert default settings if empty
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))?;

    if count == 0 {
        conn.execute(
            "INSERT INTO settings (language, theme, telemetry_enabled, first_run, autostart) VALUES ('en', 'system', 0, 1, 1)",
            [],
        )?;
    }

    DB.set(Arc::new(Mutex::new(conn)))
        .map_err(|_| rusqlite::Error::ExecuteReturnedResults)?;

    Ok(())
}

pub fn get_db() -> Arc<Mutex<Connection>> {
    DB.get().expect("Database not initialized").clone()
}

pub fn migrate_rules_to_relative() -> SqliteResult<()> {
    let folders = get_watched_folders()?;
    let db = get_db();
    let conn = db.lock().unwrap();
    for folder in folders {
        let folder_norm = folder.path.trim_end_matches('/').trim_end_matches('\\');
        if folder_norm.is_empty() {
            continue;
        }
        let mut stmt =
            conn.prepare("SELECT id, destination FROM rules WHERE destination LIKE ?1")?;
        let rows: Vec<(i64, String)> = stmt
            .query_map([format!("{}%", folder_norm)], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
            .collect::<SqliteResult<Vec<_>>>()?;
        for (id, dest) in rows {
            let relative = if dest.starts_with(&folder_norm) {
                dest[folder_norm.len()..]
                    .trim_start_matches('/')
                    .trim_start_matches('\\')
                    .to_string()
            } else {
                dest.clone()
            };
            if !relative.is_empty() && relative != dest {
                conn.execute(
                    "UPDATE rules SET destination = ?1 WHERE id = ?2",
                    params![relative, id],
                )?;
            }
        }
    }
    Ok(())
}

pub fn insert_default_rules(_folder_path: &str) -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();

    // Only insert defaults if no rules exist yet
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM rules", [], |row| row.get(0))?;
    if count > 0 {
        return Ok(());
    }

    let defaults = vec![
        (
            "Images",
            1,
            vec!["jpg", "jpeg", "png", "gif", "webp", "bmp", "svg", "ico"],
            "Images",
        ),
        (
            "Documents",
            2,
            vec![
                "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "txt", "rtf", "odt",
            ],
            "Documents",
        ),
        (
            "Archives",
            3,
            vec!["zip", "rar", "7z", "tar", "gz", "bz2"],
            "Archives",
        ),
        (
            "Installers",
            4,
            vec!["exe", "msi", "msix", "appx"],
            "Installers",
        ),
        (
            "Music",
            5,
            vec!["mp3", "wav", "flac", "aac", "ogg", "wma", "m4a"],
            "Music",
        ),
        (
            "Videos",
            6,
            vec!["mp4", "avi", "mkv", "mov", "wmv", "flv", "webm"],
            "Videos",
        ),
        ("Others", 99, vec!["*"], "Others"),
    ];

    for (name, priority, exts, dest) in defaults {
        let extensions = exts.join(",");
        let destination = dest.to_string();
        conn.execute(
            "INSERT INTO rules (name, priority, extensions, destination, action, folder_id, folder_path) VALUES (?1, ?2, ?3, ?4, 'move', 0, NULL)",
            params![name, priority, extensions, destination],
        )?;
    }

    Ok(())
}

pub fn get_rules() -> SqliteResult<Vec<Rule>> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, name, priority, enabled, extensions, pattern, destination, action, folder_id, folder_path FROM rules ORDER BY priority"
    )?;

    let rules = stmt
        .query_map([], |row| {
            let exts_str: String = row.get(4)?;
            Ok(Rule {
                id: row.get(0)?,
                name: row.get(1)?,
                priority: row.get(2)?,
                enabled: row.get::<_, i32>(3)? != 0,
                extensions: exts_str
                    .split(',')
                    .map(|s| s.trim().to_lowercase())
                    .collect(),
                pattern: row.get(5)?,
                destination: row.get(6)?,
                action: row.get(7)?,
                folder_id: row.get(8)?,
                folder_path: row.get(9)?,
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()?;

    Ok(rules)
}

pub fn add_rule(rule: &Rule) -> SqliteResult<i64> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let exts = rule.extensions.join(",");
    conn.execute(
        "INSERT INTO rules (name, priority, enabled, extensions, pattern, destination, action, folder_id, folder_path) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![rule.name, rule.priority, rule.enabled as i32, exts, rule.pattern, rule.destination, rule.action, rule.folder_id, rule.folder_path],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_rule(rule: &Rule) -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let exts = rule.extensions.join(",");
    conn.execute(
        "UPDATE rules SET name=?1, priority=?2, enabled=?3, extensions=?4, pattern=?5, destination=?6, action=?7, folder_id=?8, folder_path=?9 WHERE id=?10",
        params![rule.name, rule.priority, rule.enabled as i32, exts, rule.pattern, rule.destination, rule.action, rule.folder_id, rule.folder_path, rule.id],
    )?;
    Ok(())
}

pub fn delete_rule(id: i64) -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM rules WHERE id=?1", params![id])?;
    Ok(())
}

pub fn delete_all_rules() -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM rules", [])?;
    Ok(())
}

pub fn add_rule_record(rule: &Rule, preserve_id: bool) -> SqliteResult<i64> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let exts = rule.extensions.join(",");
    if preserve_id {
        conn.execute(
            "INSERT OR REPLACE INTO rules (id, name, priority, enabled, extensions, pattern, destination, action, folder_id, folder_path) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![rule.id, rule.name, rule.priority, rule.enabled as i32, exts, rule.pattern, rule.destination, rule.action, rule.folder_id, rule.folder_path],
        )?;
    } else {
        conn.execute(
            "INSERT INTO rules (name, priority, enabled, extensions, pattern, destination, action, folder_id, folder_path) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![rule.name, rule.priority, rule.enabled as i32, exts, rule.pattern, rule.destination, rule.action, rule.folder_id, rule.folder_path],
        )?;
    }
    Ok(conn.last_insert_rowid())
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

pub fn get_settings() -> SqliteResult<AppSettings> {
    let db = get_db();
    let conn = db.lock().unwrap();
    conn.query_row(
        "SELECT id, language, theme, telemetry_enabled, first_run, autostart, grace_period_seconds, lock_check_enabled, schedule_enabled, schedule_times_per_day, schedule_time_1, schedule_time_2, schedule_time_3, schedule_time_4, schedule_cron_enabled, schedule_cron_expr, keepalive_enabled, keepalive_interval_minutes, mcp_enabled, mcp_allow_write, mcp_transport, mcp_server_name, mcp_command, mcp_args, mcp_http_url, mcp_token FROM settings LIMIT 1",
        [],
        |row| {
            Ok(AppSettings {
                id: row.get(0)?,
                language: row.get(1)?,
                theme: row.get(2)?,
                telemetry_enabled: row.get::<_, i32>(3)? != 0,
                first_run: row.get::<_, i32>(4)? != 0,
                autostart: row.get::<_, i32>(5).unwrap_or(1) != 0,
                grace_period_seconds: row.get::<_, i64>(6).unwrap_or(300),
                lock_check_enabled: row.get::<_, i32>(7).unwrap_or(1) != 0,
                schedule_enabled: row.get::<_, i32>(8).unwrap_or(0) != 0,
                schedule_times_per_day: row.get::<_, i64>(9).unwrap_or(1),
                schedule_time_1: row.get(10).ok(),
                schedule_time_2: row.get(11).ok(),
                schedule_time_3: row.get(12).ok(),
                schedule_time_4: row.get(13).ok(),
                schedule_cron_enabled: row.get::<_, i32>(14).unwrap_or(0) != 0,
                schedule_cron_expr: row.get(15).ok(),
                keepalive_enabled: row.get::<_, i32>(16).unwrap_or(0) != 0,
                keepalive_interval_minutes: row.get::<_, i64>(17).unwrap_or(15).clamp(1, 1440),
                mcp_enabled: row.get::<_, i32>(18).unwrap_or(0) != 0,
                mcp_allow_write: row.get::<_, i32>(19).unwrap_or(0) != 0,
                mcp_transport: row
                    .get::<_, String>(20)
                    .unwrap_or_else(|_| default_mcp_transport()),
                mcp_server_name: row
                    .get::<_, String>(21)
                    .unwrap_or_else(|_| default_mcp_server_name()),
                mcp_command: row.get(22).ok(),
                mcp_args: row.get(23).ok(),
                mcp_http_url: row.get(24).ok(),
                mcp_token: row.get(25).ok(),
            })
        },
    )
}

pub fn update_settings(settings: &AppSettings) -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let target_id = conn
        .query_row("SELECT id FROM settings LIMIT 1", [], |row| row.get(0))
        .unwrap_or(settings.id.unwrap_or(1));
    conn.execute(
        "UPDATE settings SET language=?1, theme=?2, telemetry_enabled=?3, first_run=?4, autostart=?5, grace_period_seconds=?6, lock_check_enabled=?7, schedule_enabled=?8, schedule_times_per_day=?9, schedule_time_1=?10, schedule_time_2=?11, schedule_time_3=?12, schedule_time_4=?13, schedule_cron_enabled=?14, schedule_cron_expr=?15, keepalive_enabled=?16, keepalive_interval_minutes=?17, mcp_enabled=?18, mcp_allow_write=?19, mcp_transport=?20, mcp_server_name=?21, mcp_command=?22, mcp_args=?23, mcp_http_url=?24, mcp_token=?25 WHERE id=?26",
        params![
            settings.language,
            settings.theme,
            settings.telemetry_enabled as i32,
            settings.first_run as i32,
            settings.autostart as i32,
            settings.grace_period_seconds,
            settings.lock_check_enabled as i32,
            settings.schedule_enabled as i32,
            settings.schedule_times_per_day,
            settings.schedule_time_1,
            settings.schedule_time_2,
            settings.schedule_time_3,
            settings.schedule_time_4,
            settings.schedule_cron_enabled as i32,
            settings.schedule_cron_expr,
            settings.keepalive_enabled as i32,
            settings.keepalive_interval_minutes.clamp(1, 1440),
            settings.mcp_enabled as i32,
            settings.mcp_allow_write as i32,
            settings.mcp_transport,
            settings.mcp_server_name,
            settings.mcp_command,
            settings.mcp_args,
            settings.mcp_http_url,
            settings.mcp_token,
            target_id
        ],
    )?;
    Ok(())
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

pub fn clear_logs() -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM action_logs", [])?;
    Ok(())
}

pub fn get_config_snapshot() -> SqliteResult<ConfigSnapshot> {
    Ok(ConfigSnapshot {
        settings: get_settings()?,
        folders: get_watched_folders()?,
        rules: get_rules()?,
    })
}

pub fn import_config_snapshot(snapshot: &ConfigSnapshot, replace: bool) -> SqliteResult<()> {
    update_settings(&snapshot.settings)?;

    if replace {
        delete_all_rules()?;
        delete_all_watched_folders()?;
        for folder in &snapshot.folders {
            add_watched_folder_record(folder, true)?;
        }
        for rule in &snapshot.rules {
            add_rule_record(rule, true)?;
        }
    } else {
        for folder in &snapshot.folders {
            add_watched_folder_record(folder, false)?;
        }
        for rule in &snapshot.rules {
            let mut imported = rule.clone();
            imported.id = None;
            imported.folder_id = 0;
            imported.folder_path = imported
                .folder_path
                .filter(|value| !value.trim().is_empty());
            add_rule_record(&imported, false)?;
        }
    }

    Ok(())
}

fn parse_utc_ts(s: String) -> SqliteResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })
}

pub fn list_orden_config_names() -> SqliteResult<Vec<String>> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare("SELECT name FROM orden_configs ORDER BY name ASC")?;
    let rows: SqliteResult<Vec<String>> = stmt.query_map([], |row| row.get(0))?.collect();
    rows
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

pub fn delete_orden_config(name: &str) -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM orden_configs WHERE name=?1", params![name])?;
    conn.execute("DELETE FROM orden_jobs WHERE config_name=?1", params![name])?;
    Ok(())
}

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
