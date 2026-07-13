pub mod cli;
pub mod commands;
pub mod db;
pub mod i18n;
pub mod ignore;
pub mod mcp;
pub mod orden;
pub mod orden_jobs;
pub mod orden_runtime;
pub mod rules;
pub mod scheduler;
pub mod tray;
pub mod updater;
pub mod watcher;

use commands::*;
use db::{init_db, FOLDER_MODE_SILENT};
use directories::ProjectDirs;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;
use watcher::FolderWatcher;

pub struct AppState {
    pub watcher: Arc<Mutex<FolderWatcher>>,
    pub ignored_files: Arc<Mutex<HashMap<String, Instant>>>,
    /// Last destination folder waiting to be opened when app is activated by notification click
    pub pending_open_folder: Arc<Mutex<Option<String>>>,
    pub scheduler: scheduler::Scheduler,
}

pub fn try_run_cli() -> bool {
    cli::try_run_from_env()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let ignored_files = Arc::new(Mutex::new(HashMap::new()));
    let pending_open_folder: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // When Windows activates the app (e.g. user clicked a notification),
            // open any pending folder first, then show the main settings window.
            if let Some(state) = app.try_state::<AppState>() {
                let folder = state.pending_open_folder.lock().unwrap().take();
                if let Some(path) = folder {
                    // Open the destination folder in Explorer robustly
                    #[cfg(target_os = "windows")]
                    {
                        let _ = std::process::Command::new("cmd")
                            .args(["/c", "start", "", &path])
                            .spawn();
                    }
                    #[cfg(target_os = "macos")]
                    {
                        let _ = std::process::Command::new("open").arg(&path).spawn();
                    }
                    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
                    {
                        let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
                    }
                }
            }
            tray::show_settings_window(app);
        }))
        .manage(AppState {
            watcher: Arc::new(Mutex::new(FolderWatcher::new(
                ignored_files.clone(),
                pending_open_folder.clone(),
            ))),
            ignored_files,
            pending_open_folder,
            scheduler: scheduler::Scheduler::new(),
        })
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Initialize database
            if let Some(proj_dirs) = ProjectDirs::from("cc", "shelfy", "shelfy") {
                let data_dir = proj_dirs.data_dir().to_path_buf();
                std::fs::create_dir_all(&data_dir).ok();
                init_db(data_dir.clone()).expect("Failed to initialize database");
            }

            // Initialize default rules on first run
            if let Ok(settings) = db::get_settings() {
                if settings.first_run {
                    let downloads = commands::get_downloads_folder();
                    let _ = db::add_watched_folder(&downloads, FOLDER_MODE_SILENT);
                    let _ = db::insert_default_rules(&downloads);
                    let mut new_settings = settings;
                    new_settings.first_run = false;
                    let _ = db::update_settings(&new_settings);
                }
            }

            // Setup system tray
            let tray_lang = db::get_settings()
                .map(|s| s.language)
                .unwrap_or_else(|_| "en".to_string());
            tray::setup_tray(&app_handle, &tray_lang)?;

            // Opening Shelfy itself goes straight to its main settings window.
            // Autostart remains silent so it can continue running as a tray app.
            if !std::env::args().any(|arg| arg == "--autostart") {
                tray::show_settings_window(&app_handle);
            }

            // Sync autostart with user settings
            if let Ok(settings) = db::get_settings() {
                let auto_manager = app.autolaunch();
                if settings.autostart {
                    let _ = auto_manager.enable();
                } else {
                    let _ = auto_manager.disable();
                }
            }

            // Start folder watcher
            let state = app.state::<AppState>();
            {
                let mut watcher = state.watcher.lock().unwrap();
                let _ = watcher.watch_folders(app_handle.clone());
            }

            // Start scheduled-clean background thread
            state.scheduler.start(app_handle.clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_system_language,
            get_rules_cmd,
            add_rule_cmd,
            update_rule_cmd,
            delete_rule_cmd,
            get_folders_cmd,
            add_folder_cmd,
            remove_folder_cmd,
            update_folder_mode_cmd,
            get_logs_cmd,
            get_stats_cmd,
            undo_action_cmd,
            undo_all_cmd,
            get_settings_cmd,
            update_settings_cmd,
            enable_autostart_cmd,
            disable_autostart_cmd,
            is_autostart_enabled_cmd,
            clear_logs_cmd,
            delete_log_cmd,
            scan_folder_cmd,
            open_folder_cmd,
            validate_folder_access_cmd,
            open_full_disk_access_settings_cmd,
            get_downloads_folder,
            initialize_defaults_cmd,
            close_popup,
            close_settings,
            show_notification,
            load_shelfyignore_cmd,
            save_shelfyignore_cmd,
            get_pending_open_folder_cmd,
            show_popup_cmd,
            show_settings_cmd,
            get_pending_files_cmd,
            refresh_watcher_cmd,
            get_schedule_cmd,
            update_schedule_cmd,
            validate_cron_cmd,
            get_scheduler_logs_cmd,
            clear_scheduler_logs_cmd,
            system_keepalive_status_cmd,
            install_system_keepalive_cmd,
            uninstall_system_keepalive_cmd,
            mcp_client_config_cmd,
            mcp_help_cmd,
            export_rules_cmd,
            import_rules_cmd,
            export_config_cmd,
            import_config_cmd,
            orden_list_cmd,
            orden_load_cmd,
            orden_save_cmd,
            orden_rename_cmd,
            orden_duplicate_cmd,
            orden_delete_cmd,
            orden_template_list_cmd,
            orden_template_load_cmd,
            orden_template_save_cmd,
            orden_template_delete_cmd,
            orden_check_cmd,
            orden_visual_from_yaml_cmd,
            orden_run_cmd,
            orden_task_status_cmd,
            orden_history_cmd,
            orden_delete_history_cmd,
            orden_clear_history_cmd,
            orden_jobs_cmd,
            orden_save_job_cmd,
            orden_delete_job_cmd,
            orden_run_job_cmd,
            check_update_cmd,
            install_update_cmd,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::WindowEvent { label, event, .. } = &event {
                if label == "settings" {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Some(window) = app.get_webview_window(label) {
                            let _ = window.hide();
                        }
                    }
                }
            }

            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                tray::show_settings_window(app);
            }
        });
}
