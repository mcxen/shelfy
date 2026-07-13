use crate::AppState;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

#[derive(serde::Serialize)]
pub struct FolderAccessStatus {
    path: String,
    exists: bool,
    is_directory: bool,
    readable: bool,
    permission_denied: bool,
    error: Option<String>,
}

#[tauri::command]
pub fn open_folder_cmd(path: String) -> Result<(), String> {
    if path.is_empty() {
        return Err("Path is empty".to_string());
    }
    eprintln!("[open_folder_cmd] opening: {}", path);
    #[cfg(target_os = "windows")]
    {
        if path.starts_with("http://") || path.starts_with("https://") {
            std::process::Command::new("cmd")
                .args(["/c", "start", "", &path])
                .spawn()
                .map_err(|e| e.to_string())?;
        } else {
            // Normalize to backslashes - Windows Explorer requires them
            let win_path = path.replace('/', "\\");
            std::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    &format!("explorer '{}'", win_path),
                ])
                .spawn()
                .map_err(|e| e.to_string())?;
        }
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn validate_folder_access_cmd(path: String) -> FolderAccessStatus {
    let target = std::path::Path::new(&path);
    let metadata = std::fs::metadata(target);
    let (exists, is_directory, metadata_error) = match metadata {
        Ok(metadata) => (true, metadata.is_dir(), None),
        Err(error) => (
            false,
            false,
            Some(format!("Cannot access '{}': {}", path, error)),
        ),
    };

    if !exists || !is_directory {
        let error = metadata_error.or_else(|| Some(format!("'{}' is not a directory", path)));
        let permission_denied = error
            .as_deref()
            .map(|message| message.to_lowercase().contains("permission denied"))
            .unwrap_or(false);
        return FolderAccessStatus {
            path,
            exists,
            is_directory,
            readable: false,
            permission_denied,
            error,
        };
    }

    match std::fs::read_dir(target) {
        Ok(_) => FolderAccessStatus {
            path,
            exists: true,
            is_directory: true,
            readable: true,
            permission_denied: false,
            error: None,
        },
        Err(error) => FolderAccessStatus {
            path: path.clone(),
            exists: true,
            is_directory: true,
            readable: false,
            permission_denied: error.kind() == std::io::ErrorKind::PermissionDenied,
            error: Some(format!("Cannot read '{}': {}", path, error)),
        },
    }
}

#[tauri::command]
pub fn open_full_disk_access_settings_cmd() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles")
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("Full Disk Access is only available on macOS".to_string())
    }
}

#[tauri::command]
pub fn close_popup(app: AppHandle) {
    if let Some(window) = app.get_webview_window("popup") {
        let _ = window.hide();
    }
}

#[tauri::command]
pub fn close_settings(app: AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.hide();
    }
}

#[tauri::command]
pub fn show_notification(app: AppHandle, title: String, body: String) {
    let _ = app.notification().builder().title(title).body(body).show();
}

/// Returns and clears the pending folder path that should be opened after a notification click.
/// The frontend calls this when the popup is shown/focused to handle the in-app flow as a backup.
#[tauri::command]
pub fn get_pending_open_folder_cmd(state: tauri::State<AppState>) -> Option<String> {
    state.pending_open_folder.lock().unwrap().take()
}

#[tauri::command]
pub fn show_popup_cmd(app: AppHandle) {
    crate::tray::show_popup_window(&app);
}

#[tauri::command]
pub fn show_settings_cmd(app: AppHandle, section: Option<String>) {
    crate::tray::show_settings_window_at(&app, section.as_deref());
}
