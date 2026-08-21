// src/lib.rs
mod config;
mod error_reporter;
mod logger;

use config::AppConfig;
use error_reporter::ErrorReporter;
use logger::Logger;

use std::sync::Mutex;
use tauri::{
    command,
    menu::{Menu, MenuItem},
    tray::{MouseButton, TrayIconBuilder},
    Manager,
};
// FIX 1: Import GlobalShortcutExt
use tauri_plugin_global_shortcut::GlobalShortcutExt;

pub struct AppState {
    pub reporter: Mutex<ErrorReporter>,
    pub config: Mutex<AppConfig>,
}

#[command]
fn set_server_url(state: tauri::State<AppState>, url: String) -> Result<(), String> {
    let mut reporter = state.reporter.lock().map_err(|e| e.to_string())?;
    reporter.update_server_url(&url);

    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.server_url = url;
    config.save()?;

    Ok(())
}

#[command]
#[command]
fn report_error(
    state: tauri::State<AppState>,
    error_type: String,
    error_message: String,
    context: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let reporter = state.reporter.lock().map_err(|e| e.to_string())?;
    let result = reporter.report_error(&error_type, &error_message, None, context, None);

    match result {
        Ok(response) => Ok(serde_json::json!({
            "success": true,
            "report_id": response.id,
        })),
        Err(e) => Err(e),
    }
}

#[command]
fn submit_user_report(
    state: tauri::State<AppState>,
    description: String,
    include_logs: bool,
) -> Result<serde_json::Value, String> {
    let logger = Logger::new();
    let logs_content = if include_logs {
        logger.get_logs(100)
    } else {
        "Logs excluded by user".to_string()
    };

    let context = serde_json::json!({
        "user_description": description,
        "logs_included": include_logs,
        "logs_snippet": logs_content,
    });

    let reporter = state.reporter.lock().map_err(|e| e.to_string())?;
    let result = reporter.report_error(
        "user_report",
        "Bruger har indsendt en fejlrapport",
        None,
        Some(context),
        None,
    );

    match result {
        Ok(response) => Ok(serde_json::json!({
            "success": true,
            "report_id": response.id,
            "message": "Tak for din rapport. Vi vil se på det hurtigst muligt.",
        })),
        Err(e) => Err(e),
    }
}

fn toggle_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if let Ok(Some(monitor)) = window
            .current_monitor()
            .map(|m| m.or_else(|| window.primary_monitor().ok().flatten()))
        {
            let monitor_size = monitor.size();
            if let Ok(pos) = window.outer_position() {
                if pos.x < 0
                    || pos.x > monitor_size.width as i32
                    || pos.y < 0
                    || pos.y > monitor_size.height as i32
                {
                    if let Err(e) = window.center() {
                        eprintln!("Kunne ikke centrere vindue: {e}");
                    }
                }
            }
        }

        let is_visible = window.is_visible().unwrap_or(false);
        if is_visible {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = AppConfig::load();
    let server_url = config.server_url.clone();

    let state = AppState {
        reporter: Mutex::new(ErrorReporter::new(&server_url)),
        config: Mutex::new(config),
    };

    tauri::Builder::default()
        .manage(state)
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            // Global shortcut
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            {
                use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};
                let shortcut =
                    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyE);
                // FIX: Brug app.handle() i stedet for app direkte
                if let Err(e) = app.handle().global_shortcut().register(shortcut) {
                    eprintln!("Advarsel: Kunne ikke registrere global genvej Ctrl+Shift+E: {e}");
                }
            }

            // System tray
            let quit_item = MenuItem::with_id(app, "quit", "Afslut", true, None::<&str>)?;
            let toggle_item = MenuItem::with_id(app, "toggle", "Åbn Eira", true, None::<&str>)?;
            let hide_item = MenuItem::with_id(app, "hide", "Skjul", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&toggle_item, &hide_item, &quit_item])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "toggle" => {
                        toggle_main_window(app);
                    }
                    "hide" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
                    _ => {}
                })
                // FIX 2: Brug pattern matching i stedet for event.button
                .on_tray_icon_event(|tray, event| {
                    match event {
                        tauri::tray::TrayIconEvent::Click {
                            button: MouseButton::Left,
                            ..
                        } => {
                            let app_handle = tray.app_handle();
                            toggle_main_window(&app_handle);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_server_url,
            report_error,
            submit_user_report,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
