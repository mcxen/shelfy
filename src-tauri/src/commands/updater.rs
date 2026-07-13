use tauri::AppHandle;

#[tauri::command]
pub async fn check_update_cmd() -> Result<crate::updater::UpdateInfo, String> {
    tauri::async_runtime::spawn_blocking(crate::updater::check_update)
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn install_update_cmd(
    app: AppHandle,
    info: crate::updater::UpdateInfo,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || crate::updater::download_and_install(&app, &info))
        .await
        .map_err(|error| error.to_string())?
}
