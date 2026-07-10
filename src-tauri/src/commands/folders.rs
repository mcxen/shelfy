use crate::db::{
    add_watched_folder, get_watched_folders, insert_default_rules, is_folder_auto_mode,
    is_folder_manual_mode, is_folder_paused_mode, is_valid_folder_mode, remove_watched_folder,
    update_folder_mode, WatchedFolder, FOLDER_MODE_SILENT,
};
use crate::ignore::{load_shelfyignore, save_shelfyignore};
use crate::rules::manual_scan_folder;
use crate::AppState;
use tauri::Manager;

#[tauri::command]
pub fn get_folders_cmd() -> Result<Vec<WatchedFolder>, String> {
    get_watched_folders().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_folder_cmd(app: tauri::AppHandle, path: String, mode: String) -> Result<i64, String> {
    let metadata = std::fs::metadata(&path)
        .map_err(|error| format!("Cannot access watched folder '{}': {}", path, error))?;
    if !metadata.is_dir() {
        return Err(format!("Watched path is not a folder: {}", path));
    }
    std::fs::read_dir(&path)
        .map_err(|error| format!("Cannot read watched folder '{}': {}", path, error))?;
    let id = add_watched_folder(&path, &mode).map_err(|e| e.to_string())?;
    if let Some(state) = app.try_state::<crate::AppState>() {
        let mut watcher = state.watcher.lock().unwrap();
        let _ = watcher.refresh(app.clone());
    }
    crate::tray::refresh_tray_menu(&app);
    Ok(id)
}

#[tauri::command]
pub fn remove_folder_cmd(app: tauri::AppHandle, id: i64) -> Result<(), String> {
    remove_watched_folder(id).map_err(|e| e.to_string())?;
    if let Some(state) = app.try_state::<crate::AppState>() {
        let mut watcher = state.watcher.lock().unwrap();
        let _ = watcher.refresh(app.clone());
    }
    crate::tray::refresh_tray_menu(&app);
    Ok(())
}

#[tauri::command]
pub fn update_folder_mode_cmd(app: tauri::AppHandle, id: i64, mode: String) -> Result<(), String> {
    if !is_valid_folder_mode(&mode) {
        return Err(format!("Invalid folder mode: {}", mode));
    }

    let old_mode = get_watched_folders()
        .ok()
        .and_then(|folders| folders.into_iter().find(|f| f.id == Some(id)))
        .map(|f| f.mode);

    update_folder_mode(id, &mode).map_err(|e| e.to_string())?;

    if let Some(state) = app.try_state::<crate::AppState>() {
        let mut watcher = state.watcher.lock().unwrap();

        // If switching from manual to silent, flush collected files into the auto queue.
        if is_folder_auto_mode(&mode) {
            if let Some(ref old) = old_mode {
                if is_folder_manual_mode(old) {
                    if let Some(folder) = get_watched_folders()
                        .ok()
                        .and_then(|folders| folders.into_iter().find(|f| f.id == Some(id)))
                    {
                        watcher.flush_manual_to_pending(&folder.path);
                    }
                }
            }
        }

        let _ = watcher.refresh(app.clone());
    }
    crate::tray::refresh_tray_menu(&app);
    Ok(())
}

#[tauri::command]
pub fn scan_folder_cmd(path: String) -> Result<Vec<(String, String, String)>, String> {
    let folders = get_watched_folders().map_err(|e| e.to_string())?;
    if folders
        .iter()
        .find(|f| f.path == path)
        .map(|f| is_folder_paused_mode(&f.mode))
        .unwrap_or(false)
    {
        return Ok(vec![]);
    }
    manual_scan_folder(&path)
}

#[tauri::command]
pub fn get_downloads_folder() -> String {
    directories::UserDirs::new()
        .and_then(|d| d.download_dir().map(|p| p.to_string_lossy().to_string()))
        .unwrap_or_else(|| {
            if cfg!(target_os = "windows") {
                "C:/Users".to_string()
            } else {
                std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string())
            }
        })
}

#[tauri::command]
pub fn initialize_defaults_cmd() -> Result<(), String> {
    let downloads = get_downloads_folder();
    add_watched_folder(&downloads, FOLDER_MODE_SILENT).map_err(|e| e.to_string())?;
    insert_default_rules(&downloads).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn load_shelfyignore_cmd(folder_path: String) -> Result<Vec<String>, String> {
    Ok(load_shelfyignore(&folder_path))
}

#[tauri::command]
pub fn save_shelfyignore_cmd(folder_path: String, patterns: Vec<String>) -> Result<(), String> {
    save_shelfyignore(&folder_path, &patterns)
}

/// Return files detected in manual-mode folders that are waiting for Clean Now.
#[tauri::command]
pub fn get_pending_files_cmd(
    state: tauri::State<AppState>,
) -> Result<Vec<(String, String)>, String> {
    let watcher = state.watcher.lock().unwrap();
    Ok(watcher.get_pending_files())
}

/// Refresh all folder watchers. Useful after changing folder modes.
#[tauri::command]
pub fn refresh_watcher_cmd(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(state) = app.try_state::<crate::AppState>() {
        let mut watcher = state.watcher.lock().unwrap();
        watcher.refresh(app.clone()).map_err(|e| e.to_string())?;
    }
    Ok(())
}
