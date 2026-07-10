use crate::db::{get_config_snapshot, import_config_snapshot, ConfigSnapshot};
use tauri::Manager;

/// Export settings, watched folders, and rules as a JSON config snapshot.
#[tauri::command]
pub fn export_config_cmd(path: String) -> Result<(), String> {
    let snapshot = get_config_snapshot().map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&snapshot).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

/// Import a JSON config snapshot.
#[tauri::command]
pub fn import_config_cmd(app: tauri::AppHandle, path: String, replace: bool) -> Result<(), String> {
    let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let snapshot: ConfigSnapshot = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    import_config_snapshot(&snapshot, replace).map_err(|e| e.to_string())?;

    if let Some(state) = app.try_state::<crate::AppState>() {
        let mut watcher = state.watcher.lock().unwrap();
        watcher.refresh(app.clone()).map_err(|e| e.to_string())?;
    }

    Ok(())
}
