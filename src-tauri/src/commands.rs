use crate::db::*;
use crate::ignore::{load_shelfyignore, save_shelfyignore};
use crate::rules::manual_scan_folder;
use crate::AppState;
use std::time::Instant;
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_notification::NotificationExt;

/// Resolve the Shelfy data directory (where shelfy.db / orden configs live).
fn data_dir() -> Result<std::path::PathBuf, String> {
    directories::ProjectDirs::from("cc", "shelfy", "shelfy")
        .map(|p| p.data_dir().to_path_buf())
        .ok_or_else(|| "Unable to resolve data directory".to_string())
}

/// A captured log entry from an orden run (mirrors `orden::action::LogEntry`).
#[derive(serde::Serialize)]
pub struct OrdenLog {
    level: String,
    sender: String,
    rule_nr: i64,
    path: String,
    msg: String,
}

#[derive(serde::Serialize)]
pub struct OrdenRunResult {
    success: u64,
    errors: u64,
    simulate: bool,
    logs: Vec<OrdenLog>,
}

#[derive(serde::Serialize)]
pub struct SystemKeepaliveStatus {
    supported: bool,
    platform: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct OrdenVisualConfig {
    rules: Vec<OrdenVisualRule>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct OrdenVisualRule {
    id: String,
    name: String,
    enabled: bool,
    targets: String,
    location: String,
    subfolders: bool,
    extensions: String,
    #[serde(rename = "filterMode", default = "default_filter_mode")]
    filter_mode: String,
    tags: String,
    action: String,
    destination: String,
    #[serde(rename = "archiveFormat", default = "default_archive_format")]
    archive_format: String,
    #[serde(rename = "archivePassword", default)]
    archive_password: String,
    #[serde(rename = "archivePasswords", default)]
    archive_passwords: String,
    #[serde(rename = "deleteOriginal", default)]
    delete_original: bool,
    #[serde(rename = "onConflict", default = "default_on_conflict")]
    on_conflict: String,
}

fn default_filter_mode() -> String {
    "all".to_string()
}

fn default_archive_format() -> String {
    "auto".to_string()
}

fn default_on_conflict() -> String {
    "rename_new".to_string()
}

#[derive(serde::Serialize)]
pub struct McpClientConfig {
    enabled: bool,
    transport: String,
    config_json: String,
}

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
pub fn get_system_language() -> String {
    let locale = sys_locale::get_locale().unwrap_or_else(|| "en".to_string());
    let lang = locale.split('-').next().unwrap_or("en").to_lowercase();
    match lang.as_str() {
        "pl" => "pl".to_string(),
        "it" => "it".to_string(),
        "de" => "de".to_string(),
        "fr" => "fr".to_string(),
        "ru" => "ru".to_string(),
        "ja" => "ja".to_string(),
        "zh" => "zh".to_string(),
        _ => "en".to_string(),
    }
}

#[tauri::command]
pub fn get_rules_cmd() -> Result<Vec<Rule>, String> {
    get_rules().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_rule_cmd(rule: Rule) -> Result<i64, String> {
    add_rule(&rule).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_rule_cmd(rule: Rule) -> Result<(), String> {
    update_rule(&rule).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_rule_cmd(id: i64) -> Result<(), String> {
    delete_rule(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_folders_cmd() -> Result<Vec<WatchedFolder>, String> {
    get_watched_folders().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_folder_cmd(app: tauri::AppHandle, path: String, mode: String) -> Result<i64, String> {
    let _ = std::fs::create_dir_all(&path);
    let id = add_watched_folder(&path, &mode).map_err(|e| e.to_string())?;
    if let Some(state) = app.try_state::<crate::AppState>() {
        let mut watcher = state.watcher.lock().unwrap();
        let _ = watcher.refresh(app.clone());
    }
    Ok(id)
}

#[tauri::command]
pub fn remove_folder_cmd(app: tauri::AppHandle, id: i64) -> Result<(), String> {
    remove_watched_folder(id).map_err(|e| e.to_string())?;
    if let Some(state) = app.try_state::<crate::AppState>() {
        let mut watcher = state.watcher.lock().unwrap();
        let _ = watcher.refresh(app.clone());
    }
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
    Ok(())
}

#[tauri::command]
pub fn get_logs_cmd(limit: i64) -> Result<Vec<ActionLog>, String> {
    get_recent_logs(limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_stats_cmd() -> Result<Vec<(String, i64)>, String> {
    get_weekly_stats().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn undo_action_cmd(id: i64, state: tauri::State<AppState>) -> Result<bool, String> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let log: Option<(String, String, String)> = conn
        .query_row(
            "SELECT source_path, destination_path, action FROM action_logs WHERE id=?1 AND undone=0",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .ok();

    if let Some((source, dest, action)) = log {
        if !dest.is_empty() && std::path::Path::new(&dest).exists() {
            if action == "copy" {
                let _ = std::fs::remove_file(&dest);
            } else {
                let _ = std::fs::rename(&dest, &source);
                // Ignore this file for 5 seconds so the watcher doesn't re-process it
                let mut ignored = state.ignored_files.lock().unwrap();
                ignored.insert(source, Instant::now());
            }
        }
        conn.execute("UPDATE action_logs SET undone=1 WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub fn undo_all_cmd(state: tauri::State<AppState>) -> Result<i32, String> {
    let logs = crate::db::get_undoable_logs().map_err(|e| e.to_string())?;
    let db = get_db();
    let conn = db.lock().unwrap();
    let mut count = 0;

    for (id, source, dest, action) in logs {
        if !dest.is_empty() && std::path::Path::new(&dest).exists() {
            if action == "copy" {
                let _ = std::fs::remove_file(&dest);
            } else {
                let _ = std::fs::rename(&dest, &source);
                let mut ignored = state.ignored_files.lock().unwrap();
                ignored.insert(source, Instant::now());
            }
        }
        let _ = conn.execute("UPDATE action_logs SET undone=1 WHERE id=?1", [id]);
        count += 1;
    }

    Ok(count)
}

#[tauri::command]
pub fn get_settings_cmd() -> Result<AppSettings, String> {
    get_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_settings_cmd(settings: AppSettings) -> Result<(), String> {
    update_settings(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn mcp_client_config_cmd() -> Result<McpClientConfig, String> {
    let settings = get_settings().map_err(|e| e.to_string())?;
    let server_name = clean_mcp_server_name(&settings.mcp_server_name);
    let transport = clean_mcp_transport(&settings.mcp_transport);
    let config = if transport == "http" {
        let mut server = serde_json::json!({
            "url": settings
                .mcp_http_url
                .as_deref()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or("http://127.0.0.1:8765/mcp")
        });
        if let Some(token) = settings
            .mcp_token
            .as_deref()
            .filter(|v| !v.trim().is_empty())
        {
            server["token"] = serde_json::Value::String(token.to_string());
        }
        serde_json::json!({
            "mcpServers": {
                server_name: server
            }
        })
    } else {
        let command = settings
            .mcp_command
            .as_deref()
            .filter(|v| !v.trim().is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| {
                std::env::current_exe()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "shelfy".to_string())
            });
        let args = split_mcp_args(settings.mcp_args.as_deref().unwrap_or("--mcp"));
        serde_json::json!({
            "mcpServers": {
                server_name: {
                    "command": command,
                    "args": args
                }
            }
        })
    };
    Ok(McpClientConfig {
        enabled: settings.mcp_enabled,
        transport,
        config_json: serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?,
    })
}

#[tauri::command]
pub fn clear_logs_cmd() -> Result<(), String> {
    clear_logs().map_err(|e| e.to_string())
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
pub fn close_popup(app: AppHandle) {
    if let Some(window) = app.get_webview_window("popup") {
        let _ = window.hide();
    }
}

#[tauri::command]
pub fn close_settings(app: AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.close();
    }
}

#[tauri::command]
pub fn show_notification(app: AppHandle, title: String, body: String) {
    let _ = app.notification().builder().title(title).body(body).show();
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

#[tauri::command]
pub fn load_shelfyignore_cmd(folder_path: String) -> Result<Vec<String>, String> {
    Ok(load_shelfyignore(&folder_path))
}

#[tauri::command]
pub fn save_shelfyignore_cmd(folder_path: String, patterns: Vec<String>) -> Result<(), String> {
    save_shelfyignore(&folder_path, &patterns)
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
pub fn show_settings_cmd(app: AppHandle) {
    crate::tray::show_settings_window(&app);
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
pub fn refresh_watcher_cmd(app: AppHandle) -> Result<(), String> {
    if let Some(state) = app.try_state::<crate::AppState>() {
        let mut watcher = state.watcher.lock().unwrap();
        watcher.refresh(app.clone()).map_err(|e| e.to_string())?;
    }
    Ok(())
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

/// Export all rules to a JSON file at the given path.
#[tauri::command]
pub fn export_rules_cmd(path: String) -> Result<(), String> {
    let rules = get_rules().map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&rules).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

/// Import rules from a JSON file at the given path.
/// If `replace` is true, all existing rules are removed before inserting.
/// Each imported rule is inserted with `id` set to None to avoid collisions.
#[tauri::command]
pub fn import_rules_cmd(path: String, replace: bool) -> Result<usize, String> {
    let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let rules: Vec<Rule> = serde_json::from_str(&data).map_err(|e| e.to_string())?;

    if replace {
        delete_all_rules().map_err(|e| e.to_string())?;
    }

    let mut count = 0;
    for mut rule in rules {
        rule.id = None;
        add_rule(&rule).map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(count)
}

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

// ---------------------------------------------------------------------------
// Orden (advanced YAML rules engine)
// ---------------------------------------------------------------------------

/// List available orden config names.
#[tauri::command]
pub fn orden_list_cmd() -> Result<Vec<String>, String> {
    let mut names = list_orden_config_names().map_err(|e| e.to_string())?;
    for name in crate::orden::list_config_names(&data_dir()?) {
        if !names.contains(&name) {
            if let Ok(yaml) = crate::orden::load_config_text(&data_dir()?, &name) {
                let _ = upsert_orden_config(&name, &yaml);
            }
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

/// Load a config's YAML text by name.
#[tauri::command]
pub fn orden_load_cmd(name: String) -> Result<String, String> {
    if let Some(record) = get_orden_config(&name).map_err(|e| e.to_string())? {
        return Ok(record.yaml);
    }
    let yaml = crate::orden::load_config_text(&data_dir()?, &name)?;
    let _ = upsert_orden_config(&name, &yaml);
    Ok(yaml)
}

/// Save a config's YAML text by name (creates the orden dir if needed).
#[tauri::command]
pub fn orden_save_cmd(name: String, yaml: String) -> Result<(), String> {
    crate::orden::save_config_text(&data_dir()?, &name, &yaml)?;
    let clean = name
        .trim()
        .trim_end_matches(".yaml")
        .trim_end_matches(".yml")
        .to_string();
    upsert_orden_config(&clean, &yaml).map_err(|e| e.to_string())
}

/// Delete a config by name.
#[tauri::command]
pub fn orden_delete_cmd(name: String) -> Result<(), String> {
    let _ = crate::orden::delete_config(&data_dir()?, &name);
    delete_orden_config(&name).map_err(|e| e.to_string())
}

/// Validate a config's YAML text without executing it.
#[tauri::command]
pub fn orden_check_cmd(yaml: String) -> Result<(), String> {
    crate::orden::Config::from_string(&yaml).map(|_| ())
}

#[tauri::command]
pub fn orden_visual_from_yaml_cmd(yaml: String) -> Result<OrdenVisualConfig, String> {
    let value: serde_yaml::Value = serde_yaml::from_str(&yaml).map_err(|e| e.to_string())?;
    let rules = value
        .as_mapping()
        .and_then(|m| yaml_get(m, "rules"))
        .and_then(|v| v.as_sequence())
        .ok_or_else(|| "YAML must contain a rules list".to_string())?;

    let visual_rules = rules
        .iter()
        .enumerate()
        .map(|(idx, rule)| parse_visual_rule(idx, rule))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(OrdenVisualConfig {
        rules: visual_rules,
    })
}

/// Simulate or run an orden config from YAML text.
/// `simulate`: true = dry run, false = apply actions.
#[tauri::command]
pub fn orden_run_cmd(
    yaml: String,
    simulate: bool,
    tags: Vec<String>,
    skip_tags: Vec<String>,
) -> Result<OrdenRunResult, String> {
    let opts = crate::orden::ExecuteOptions {
        simulate,
        tags: tags.into_iter().collect(),
        skip_tags: skip_tags.into_iter().collect(),
        working_dir: std::env::current_dir().unwrap_or_default(),
    };
    let r = crate::orden::run_yaml(&yaml, &opts)?;
    let config_name =
        find_orden_config_name_for_yaml(&yaml).unwrap_or_else(|| "<ad-hoc>".to_string());
    let _ = log_orden_run(
        &config_name,
        simulate,
        r.success as i64,
        r.errors as i64,
        "manual",
        &serde_json::to_string(&r.logs).unwrap_or_else(|_| "[]".to_string()),
    );
    Ok(OrdenRunResult {
        success: r.success,
        errors: r.errors,
        simulate: r.simulate,
        logs: r
            .logs
            .into_iter()
            .map(|l| OrdenLog {
                level: l.level,
                sender: l.sender,
                rule_nr: l.rule_nr,
                path: l.path,
                msg: l.msg,
            })
            .collect(),
    })
}

fn clean_mcp_server_name(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "shelfy".to_string()
    } else {
        trimmed.to_string()
    }
}

fn clean_mcp_transport(value: &str) -> String {
    if value.eq_ignore_ascii_case("http") {
        "http".to_string()
    } else {
        "stdio".to_string()
    }
}

fn split_mcp_args(value: &str) -> Vec<String> {
    let args = value
        .split_whitespace()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if args.is_empty() {
        vec!["--mcp".to_string()]
    } else {
        args
    }
}

fn parse_visual_rule(idx: usize, value: &serde_yaml::Value) -> Result<OrdenVisualRule, String> {
    let mapping = value
        .as_mapping()
        .ok_or_else(|| format!("Rule {} must be a YAML mapping", idx + 1))?;
    let action = parse_first_action_name(yaml_get(mapping, "actions"));
    Ok(OrdenVisualRule {
        id: format!("rule-{}", idx + 1),
        name: yaml_get(mapping, "name")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled rule")
            .to_string(),
        enabled: yaml_get(mapping, "enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        targets: yaml_get(mapping, "targets")
            .and_then(|v| v.as_str())
            .unwrap_or("files")
            .to_string(),
        location: parse_locations(yaml_get(mapping, "locations")),
        subfolders: yaml_get(mapping, "subfolders")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        extensions: parse_extension_filters(yaml_get(mapping, "filters")),
        filter_mode: yaml_get(mapping, "filter_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("all")
            .to_string(),
        tags: parse_string_list(yaml_get(mapping, "tags")),
        destination: parse_action_destinations(yaml_get(mapping, "actions"), &action),
        archive_format: parse_action_field(yaml_get(mapping, "actions"), &action, "format", "auto"),
        archive_password: parse_action_field(yaml_get(mapping, "actions"), &action, "password", ""),
        archive_passwords: parse_action_list_field(
            yaml_get(mapping, "actions"),
            &action,
            "passwords",
        ),
        delete_original: parse_action_bool_field(
            yaml_get(mapping, "actions"),
            &action,
            "delete_original",
            false,
        ),
        on_conflict: parse_action_field(
            yaml_get(mapping, "actions"),
            &action,
            "on_conflict",
            "rename_new",
        ),
        action,
    })
}

fn yaml_get<'a>(mapping: &'a serde_yaml::Mapping, key: &str) -> Option<&'a serde_yaml::Value> {
    mapping.get(serde_yaml::Value::String(key.to_string()))
}

fn parse_locations(value: Option<&serde_yaml::Value>) -> String {
    let Some(value) = value else {
        return "~/Downloads".to_string();
    };
    let mut locations = Vec::new();
    if let Some(seq) = value.as_sequence() {
        for item in seq {
            if let Some(path) = item.as_str() {
                locations.push(path.to_string());
            } else if let Some(map) = item.as_mapping() {
                if let Some(path) = yaml_get(map, "path").and_then(|v| v.as_str()) {
                    locations.push(path.to_string());
                }
            }
        }
    } else if let Some(path) = value.as_str() {
        locations.push(path.to_string());
    }
    if locations.is_empty() {
        "~/Downloads".to_string()
    } else {
        locations.join("\n")
    }
}

fn parse_extension_filters(value: Option<&serde_yaml::Value>) -> String {
    let mut extensions = Vec::new();
    let Some(filters) = value.and_then(|v| v.as_sequence()) else {
        return String::new();
    };
    for filter in filters {
        let Some(map) = filter.as_mapping() else {
            continue;
        };
        let Some(raw) = yaml_get(map, "extension") else {
            continue;
        };
        if let Some(ext) = raw.as_str() {
            extensions.push(ext.trim_start_matches('.').to_string());
        } else if let Some(seq) = raw.as_sequence() {
            for item in seq {
                if let Some(ext) = item.as_str() {
                    extensions.push(ext.trim_start_matches('.').to_string());
                }
            }
        }
    }
    extensions.join(", ")
}

fn parse_string_list(value: Option<&serde_yaml::Value>) -> String {
    let Some(seq) = value.and_then(|v| v.as_sequence()) else {
        return String::new();
    };
    seq.iter()
        .filter_map(|v| v.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_first_action_name(value: Option<&serde_yaml::Value>) -> String {
    let Some(actions) = value.and_then(|v| v.as_sequence()) else {
        return "copy".to_string();
    };
    for action in actions {
        if let Some(map) = action.as_mapping() {
            if let Some(key) = map.keys().find_map(|k| k.as_str()) {
                return key.to_string();
            }
        }
    }
    "copy".to_string()
}

fn parse_action_destinations(value: Option<&serde_yaml::Value>, action_name: &str) -> String {
    let Some(actions) = value.and_then(|v| v.as_sequence()) else {
        return "~/Documents/Shelfy Backups/".to_string();
    };
    let mut destinations = Vec::new();
    for action in actions {
        let Some(map) = action.as_mapping() else {
            continue;
        };
        let Some((key, value)) = map.iter().next() else {
            continue;
        };
        if key.as_str() != Some(action_name) {
            continue;
        }
        destinations.extend(action_destination_values(value));
    }
    if destinations.is_empty() {
        "~/Documents/Shelfy Backups/".to_string()
    } else {
        destinations.join("\n")
    }
}

fn parse_action_mapping<'a>(
    value: Option<&'a serde_yaml::Value>,
    action_name: &str,
) -> Option<&'a serde_yaml::Mapping> {
    let actions = value.and_then(|v| v.as_sequence())?;
    for action in actions {
        let map = action.as_mapping()?;
        let (key, value) = map.iter().next()?;
        if key.as_str() == Some(action_name) {
            return value.as_mapping();
        }
    }
    None
}

fn parse_action_field(
    value: Option<&serde_yaml::Value>,
    action_name: &str,
    field: &str,
    default: &str,
) -> String {
    parse_action_mapping(value, action_name)
        .and_then(|m| yaml_get(m, field))
        .and_then(|v| v.as_str())
        .unwrap_or(default)
        .to_string()
}

fn parse_action_bool_field(
    value: Option<&serde_yaml::Value>,
    action_name: &str,
    field: &str,
    default: bool,
) -> bool {
    parse_action_mapping(value, action_name)
        .and_then(|m| yaml_get(m, field))
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}

fn parse_action_list_field(
    value: Option<&serde_yaml::Value>,
    action_name: &str,
    field: &str,
) -> String {
    let Some(raw) = parse_action_mapping(value, action_name).and_then(|m| yaml_get(m, field))
    else {
        return String::new();
    };
    if let Some(s) = raw.as_str() {
        return s.to_string();
    }
    raw.as_sequence()
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

fn action_destination_values(value: &serde_yaml::Value) -> Vec<String> {
    if let Some(dest) = value.as_str() {
        return vec![dest.to_string()];
    }
    if let Some(nested) = value.as_mapping() {
        if let Some(dest) = yaml_get(nested, "dest").and_then(|v| v.as_str()) {
            return vec![dest.to_string()];
        }
        if let Some(dest) = yaml_get(nested, "path").and_then(|v| v.as_str()) {
            return vec![dest.to_string()];
        }
        if let Some(seq) = yaml_get(nested, "dest").and_then(|v| v.as_sequence()) {
            return seq
                .iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect();
        }
    }
    Vec::new()
}

fn find_orden_config_name_for_yaml(yaml: &str) -> Option<String> {
    let names = list_orden_config_names().ok()?;
    names.into_iter().find(|name| {
        get_orden_config(name)
            .ok()
            .flatten()
            .map(|record| record.yaml == yaml)
            .unwrap_or(false)
    })
}

#[tauri::command]
pub fn orden_history_cmd(name: String, limit: i64) -> Result<Vec<OrdenRunLog>, String> {
    get_orden_run_logs(&name, limit.clamp(1, 200)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn orden_jobs_cmd() -> Result<Vec<OrdenJob>, String> {
    list_orden_jobs().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn orden_save_job_cmd(job: OrdenJob) -> Result<i64, String> {
    if job.name.trim().is_empty() {
        return Err("Job name is required".into());
    }
    if job.config_name.trim().is_empty() {
        return Err("Orden config is required".into());
    }
    if job.mode == "cron" {
        crate::scheduler::validate_cron_expression(job.cron_expr.as_deref().unwrap_or(""))?;
    }
    upsert_orden_job(&job).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn orden_delete_job_cmd(id: i64) -> Result<(), String> {
    delete_orden_job(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn orden_run_job_cmd(job: OrdenJob) -> Result<OrdenRunResult, String> {
    let yaml = get_orden_config(&job.config_name)
        .map_err(|e| e.to_string())?
        .map(|record| record.yaml)
        .ok_or_else(|| format!("Orden config '{}' not found", job.config_name))?;
    let opts = crate::orden::ExecuteOptions {
        simulate: job.simulate,
        tags: split_csv(&job.tags).into_iter().collect(),
        skip_tags: split_csv(&job.skip_tags).into_iter().collect(),
        working_dir: std::env::current_dir().unwrap_or_default(),
    };
    let r = crate::orden::run_yaml(&yaml, &opts)?;
    let _ = log_orden_run(
        &job.config_name,
        job.simulate,
        r.success as i64,
        r.errors as i64,
        "manual-job",
        &serde_json::to_string(&r.logs).unwrap_or_else(|_| "[]".to_string()),
    );
    if let Some(id) = job.id {
        let _ = mark_orden_job_run(id);
    }
    Ok(OrdenRunResult {
        success: r.success,
        errors: r.errors,
        simulate: r.simulate,
        logs: r
            .logs
            .into_iter()
            .map(|l| OrdenLog {
                level: l.level,
                sender: l.sender,
                rule_nr: l.rule_nr,
                path: l.path,
                msg: l.msg,
            })
            .collect(),
    })
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}
