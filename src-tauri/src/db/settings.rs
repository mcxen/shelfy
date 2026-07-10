use rusqlite::{params, Result as SqliteResult};
use serde::{Deserialize, Serialize};

use crate::db::get_db;

// ---------------------------------------------------------------------------
// AppSettings
// ---------------------------------------------------------------------------

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

fn default_mcp_transport() -> String {
    "stdio".to_string()
}

fn default_mcp_server_name() -> String {
    "shelfy".to_string()
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
