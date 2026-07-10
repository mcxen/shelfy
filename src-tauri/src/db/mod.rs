mod export;
mod folders;
mod history;
mod orden;
mod rules;
mod scheduler;
mod settings;

pub use export::*;
pub use folders::*;
pub use history::*;
pub use orden::*;
pub use rules::*;
pub use scheduler::*;
pub use settings::*;

use once_cell::sync::OnceCell;
use rusqlite::{Connection, Result as SqliteResult};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub(crate) static DB: OnceCell<Arc<Mutex<Connection>>> = OnceCell::new();

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

    // Migration: add missing columns to settings
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
