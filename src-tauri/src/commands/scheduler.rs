use crate::db::{
    clear_scheduler_logs, get_scheduler_logs, get_settings, update_settings, ScheduleSettings,
    SchedulerLog,
};

#[derive(serde::Serialize)]
pub struct SystemKeepaliveStatus {
    supported: bool,
    platform: String,
}

/// Get the current scheduled-clean settings.
#[tauri::command]
pub fn get_schedule_cmd() -> Result<ScheduleSettings, String> {
    let settings = get_settings().map_err(|e| e.to_string())?;
    Ok(ScheduleSettings {
        schedule_enabled: settings.schedule_enabled,
        schedule_times_per_day: settings.schedule_times_per_day,
        schedule_time_1: settings.schedule_time_1,
        schedule_time_2: settings.schedule_time_2,
        schedule_time_3: settings.schedule_time_3,
        schedule_time_4: settings.schedule_time_4,
        schedule_cron_enabled: settings.schedule_cron_enabled,
        schedule_cron_expr: settings.schedule_cron_expr,
        keepalive_enabled: settings.keepalive_enabled,
        keepalive_interval_minutes: settings.keepalive_interval_minutes,
    })
}

/// Update scheduled-clean settings.
#[tauri::command]
pub fn update_schedule_cmd(schedule: ScheduleSettings) -> Result<(), String> {
    let mut settings = get_settings().map_err(|e| e.to_string())?;
    if schedule.schedule_cron_enabled {
        let expr = schedule.schedule_cron_expr.as_deref().unwrap_or("").trim();
        crate::scheduler::validate_cron_expression(expr)?;
    }
    settings.schedule_enabled = schedule.schedule_enabled;
    settings.schedule_times_per_day = schedule.schedule_times_per_day.clamp(1, 4);
    settings.schedule_time_1 = schedule.schedule_time_1;
    settings.schedule_time_2 = schedule.schedule_time_2;
    settings.schedule_time_3 = schedule.schedule_time_3;
    settings.schedule_time_4 = schedule.schedule_time_4;
    settings.schedule_cron_enabled = schedule.schedule_cron_enabled;
    settings.schedule_cron_expr = schedule.schedule_cron_expr;
    settings.keepalive_enabled = schedule.keepalive_enabled;
    settings.keepalive_interval_minutes = schedule.keepalive_interval_minutes.clamp(1, 1440);
    update_settings(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn validate_cron_cmd(expr: String) -> Result<(), String> {
    crate::scheduler::validate_cron_expression(&expr)
}

#[tauri::command]
pub fn get_scheduler_logs_cmd(limit: i64) -> Result<Vec<SchedulerLog>, String> {
    get_scheduler_logs(limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_scheduler_logs_cmd() -> Result<(), String> {
    clear_scheduler_logs().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn system_keepalive_status_cmd() -> SystemKeepaliveStatus {
    SystemKeepaliveStatus {
        supported: cfg!(any(target_os = "windows", target_os = "macos")),
        platform: std::env::consts::OS.to_string(),
    }
}

#[tauri::command]
pub fn install_system_keepalive_cmd(interval_minutes: i64) -> Result<(), String> {
    crate::scheduler::install_system_keepalive(interval_minutes)
}

#[tauri::command]
pub fn uninstall_system_keepalive_cmd() -> Result<(), String> {
    crate::scheduler::uninstall_system_keepalive()
}
