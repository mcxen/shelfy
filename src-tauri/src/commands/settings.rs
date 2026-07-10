use crate::db::{get_settings, update_settings, AppSettings};
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
pub fn get_settings_cmd() -> Result<AppSettings, String> {
    get_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_settings_cmd(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    let language_changed = get_settings()
        .map(|current| current.language != settings.language)
        .unwrap_or(true);
    update_settings(&settings).map_err(|e| e.to_string())?;
    if language_changed {
        crate::tray::refresh_tray_menu(&app);
    }
    Ok(())
}

#[tauri::command]
pub fn enable_autostart_cmd(app: AppHandle) -> Result<(), String> {
    app.autolaunch().enable().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn disable_autostart_cmd(app: AppHandle) -> Result<(), String> {
    app.autolaunch().disable().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn is_autostart_enabled_cmd(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}
