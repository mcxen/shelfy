use crate::db::{get_settings, get_watched_folders, is_folder_paused_mode};
use crate::i18n::TrayI18n;
use crate::rules::manual_scan_folder;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

const TRAY_ICON_PNG: &[u8] = include_bytes!("../icons/tray-icon.png");

pub fn setup_tray(app: &AppHandle, lang: &str) -> Result<(), Box<dyn std::error::Error>> {
    let i18n = TrayI18n::new(lang);
    let menu = build_tray_menu(app)?;

    let mut builder = TrayIconBuilder::with_id("tray")
        .tooltip(i18n.get("tooltip"))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => {
                app.exit(0);
            }
            "open" => {
                show_popup_window(app);
            }
            "settings" => {
                show_settings_window(app);
            }
            "automation" => {
                show_settings_window_at(app, Some("advanced"));
            }
            "clean" => {
                let app = app.clone();
                std::thread::spawn(move || {
                    let _ = perform_clean(&app);
                });
            }
            "open_folder" => {
                let _ = open_primary_folder();
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } => {
                show_popup_window(tray.app_handle());
            }
            TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } => {
                show_settings_window(tray.app_handle());
            }
            _ => {}
        });

    let tray_icon = Image::from_bytes(TRAY_ICON_PNG)?;
    builder = builder.icon(tray_icon);

    #[cfg(target_os = "macos")]
    {
        builder = builder.icon_as_template(true);
    }

    let _tray = builder.build(app)?;

    Ok(())
}

fn build_tray_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    let i18n = TrayI18n::new(&tray_lang(app));
    let folders = get_watched_folders().unwrap_or_default();
    let active = folders
        .iter()
        .filter(|folder| folder.enabled && !is_folder_paused_mode(&folder.mode))
        .count();
    let status = i18n
        .get("monitoring_status")
        .replace("{active}", &active.to_string())
        .replace("{total}", &folders.len().to_string());

    let status_i = MenuItem::with_id(app, "status", status, false, None::<&str>)?;
    let open_i = MenuItem::with_id(app, "open", i18n.get("open_shelfy"), true, None::<&str>)?;
    let clean_i = MenuItem::with_id(app, "clean", i18n.get("clean_now"), true, None::<&str>)?;
    let automation_i = MenuItem::with_id(
        app,
        "automation",
        i18n.get("automation"),
        true,
        None::<&str>,
    )?;
    let open_folder_i = MenuItem::with_id(
        app,
        "open_folder",
        i18n.get("open_folder"),
        true,
        None::<&str>,
    )?;
    let settings_i = MenuItem::with_id(app, "settings", i18n.get("settings"), true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", i18n.get("quit"), true, None::<&str>)?;
    let separator_1 = PredefinedMenuItem::separator(app)?;
    let separator_2 = PredefinedMenuItem::separator(app)?;

    Ok(Menu::with_items(
        app,
        &[
            &status_i,
            &separator_1,
            &open_i,
            &clean_i,
            &automation_i,
            &open_folder_i,
            &settings_i,
            &separator_2,
            &quit_i,
        ],
    )?)
}

fn open_primary_folder() -> Result<(), String> {
    let path = get_watched_folders()
        .ok()
        .and_then(|folders| folders.into_iter().find(|folder| folder.enabled))
        .map(|folder| folder.path)
        .unwrap_or_else(crate::commands::get_downloads_folder);
    crate::commands::open_folder_cmd(path)
}

pub fn refresh_tray_menu(app: &AppHandle) {
    let Ok(menu) = build_tray_menu(app) else {
        return;
    };
    if let Some(tray) = app.tray_by_id("tray") {
        let _ = tray.set_menu(Some(menu));
        let i18n = TrayI18n::new(&tray_lang(app));
        let _ = tray.set_tooltip(Some(i18n.get("tooltip")));
    }
}

fn tray_lang(_app: &AppHandle) -> String {
    get_settings()
        .map(|s| s.language)
        .unwrap_or_else(|_| "en".to_string())
}

pub fn show_popup_window(app: &AppHandle) {
    let i18n = TrayI18n::new(&tray_lang(app));
    if let Some(window) = app.get_webview_window("popup") {
        let _ = window.show();
        let _ = window.set_focus();
    } else {
        #[cfg(target_os = "macos")]
        let window = tauri::WebviewWindowBuilder::new(
            app,
            "popup",
            tauri::WebviewUrl::App("/#/popup".into()),
        )
        .title(i18n.get("popup_title"))
        .inner_size(300.0, 420.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .shadow(false)
        .build();

        #[cfg(not(target_os = "macos"))]
        let window = tauri::WebviewWindowBuilder::new(
            app,
            "popup",
            tauri::WebviewUrl::App("/#/popup".into()),
        )
        .title(i18n.get("popup_title"))
        .inner_size(300.0, 420.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .build();

        if let Ok(win) = window {
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}

fn perform_clean(app: &AppHandle) -> Result<(), String> {
    let i18n = TrayI18n::new(&tray_lang(app));
    let folders = get_watched_folders().map_err(|e| e.to_string())?;
    let mut total = 0;
    for folder in folders {
        if !folder.enabled || is_folder_paused_mode(&folder.mode) {
            continue;
        }
        if let Ok(results) = manual_scan_folder(&folder.path) {
            total += results.len();
        }
    }
    if total > 0 {
        let msg = i18n.get("organized").replace("{}", &total.to_string());
        let _ = app.emit("show-notification", msg);
    }
    Ok(())
}

pub fn update_tray_tooltip(app: &AppHandle, count: usize) {
    let i18n = TrayI18n::new(&tray_lang(app));
    let tooltip = if count == 0 {
        i18n.get("tooltip").to_string()
    } else if count == 1 {
        i18n.get("tooltip_one_pending").replace("{}", "1")
    } else {
        i18n.get("tooltip_many_pending")
            .replace("{}", &count.to_string())
    };
    if let Some(tray) = app.tray_by_id("tray") {
        let _ = tray.set_tooltip(Some(&tooltip));
    }
}

pub fn show_settings_window(app: &AppHandle) {
    show_settings_window_at(app, None);
}

pub fn show_settings_window_at(app: &AppHandle, section: Option<&str>) {
    let i18n = TrayI18n::new(&tray_lang(app));
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
        if let Some(section) = section {
            let _ = window.emit("settings-navigate", section);
        }
    } else {
        let url = section
            .map(|section| format!("/#/settings?tab={section}"))
            .unwrap_or_else(|| "/#/settings".to_string());
        let window =
            tauri::WebviewWindowBuilder::new(app, "settings", tauri::WebviewUrl::App(url.into()))
                .title(i18n.get("settings_title"))
                .inner_size(900.0, 650.0)
                .min_inner_size(700.0, 500.0)
                .transparent(true);

        #[cfg(target_os = "macos")]
        let window = window
            .title_bar_style(tauri::TitleBarStyle::Overlay)
            .hidden_title(true)
            .traffic_light_position(tauri::LogicalPosition::new(18.0, 18.0));

        let window = window.build();

        if let Ok(win) = window {
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}
