use tauri::AppHandle;

#[tauri::command]
pub async fn check_update_cmd() -> Result<crate::updater::UpdateInfo, String> {
    tauri::async_runtime::spawn_blocking(crate::updater::check_update)
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn install_update_cmd(app: AppHandle) -> Result<(), String> {
    let app_for_exit = app.clone();
    tauri::async_runtime::spawn_blocking(move || crate::updater::download_and_install(&app))
        .await
        .map_err(|error| error.to_string())??;
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(350));
        app_for_exit.exit(0);
    });
    Ok(())
}
