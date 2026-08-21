mod config;
mod logger;
mod error_reporter;

use config::AppConfig;
use logger::Logger;
use error_reporter::ErrorReporter;

use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
    command,
    WindowEvent,
};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

pub struct AppState {
    pub logger: Mutex<Logger>,
    pub reporter: ErrorReporter,
}

#[command]
fn get_server_url() -> String {
    let config = AppConfig::load();
    config.server_url
}

#[cfg(debug_assertions)]
#[command]
fn set_server_url(url: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut config = AppConfig::load();
    config.server_url = url.clone();
    config.save()?;

    state.reporter.update_server_url(&url);
    Ok(())
}

#[command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").into()
}

#[command]
async fn report_error_cmd(
    state: tauri::State<'_, AppState>,
    error_type: String,
    error_message: String,
    context: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let result = state.reporter
        .report_error(
            &error_type,
            &error_message,
            None,
            context,
            None,
        )
        .await;

    match result {
        Ok(response) => Ok(serde_json::json!({
            "success": true,
            "report_id": response.id,
        })),
        Err(e) => {
            if let Ok(mut logger) = state.logger.lock() {
                logger.log("error", "error_reporter", &e, None);
            }
            Err(e)
        }
    }
}

#[command]
fn get_logs_cmd(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let logger = match state.logger.lock() {
        Ok(l) => l,
        Err(_) => return Err("Kunne ikke få adgang til logger state".into()),
    };
    Ok(logger.get_logs(100))
}

#[command]
async fn submit_user_report(
    state: tauri::State<'_, AppState>,
    description: String,
    include_logs: bool,
) -> Result<serde_json::Value, String> {
    let logs_content = if include_logs {
        if let Ok(logger) = state.logger.lock() {
            Some(logger.get_logs(50))
        } else {
            None
        }
    } else {
        None
    };

    let context = serde_json::json!({
        "user_description": description,
        "logs_included": include_logs,
        "logs_snippet": logs_content,
    });

    let result = state.reporter
        .report_error(
            "user_report",
            "Bruger har indsendt en fejlrapport",
            None,
            Some(context),
            None,
        )
        .await;

    match result {
        Ok(response) => Ok(serde_json::json!({
            "success": true,
            "report_id": response.id,
            "message": "Tak for din fejlrapport. Den er sendt til support.",
        })),
        Err(e) => {
            if let Ok(mut logger) = state.logger.lock() {
                logger.log("error", "user_report", &e, None);
            }
            Err(e)
        }
    }
}

fn toggle_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if let Ok(Some(monitor)) = window.current_monitor().map(|m| m.or_else(|| window.primary_monitor().ok().flatten())) {
            let monitor_size = monitor.size();
            if let Ok(pos) = window.outer_position() {
                if pos.x < 0 || pos.x > monitor_size.width as i32 || pos.y < 0 || pos.y > monitor_size.height as i32 {
                    if let Err(e) = window.center() {
                        eprintln!("Kunne ikke centrere vindue: {e}");
                    }
                }
            }
        }

        match window.is_visible() {
            Ok(true) => {
                if let Err(e) = window.hide() {
                    eprintln!("Fejl ved skjul af vindue: {e}");
                }
            }
            _ => {
                if let Err(e) = window.show() {
                    eprintln!("Fejl ved visning af vindue: {e}");
                }
                if let Err(e) = window.set_focus() {
                    eprintln!("Fejl ved fokusering af vindue: {e}");
                }
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let initial_config = AppConfig::load();
    let logger = Logger::new();
    let reporter = ErrorReporter::new(&initial_config.server_url);

    let app_state = AppState {
        logger: Mutex::new(logger),
        reporter,
    };

    let builder = tauri::Builder::default()
        .manage(app_state)
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    use tauri_plugin_global_shortcut::ShortcutState;
                    if matches!(event.state(), ShortcutState::Pressed) {
                        toggle_main_window(app);
                    }
                })
                .build(),
        );

    #[cfg(debug_assertions)]
    let builder = builder.invoke_handler(tauri::generate_handler![
        get_server_url,
        set_server_url,
        get_app_version,
        report_error_cmd,
        get_logs_cmd,
        submit_user_report
    ]);

    #[cfg(not(debug_assertions))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        get_server_url,
        get_app_version,
        report_error_cmd,
        get_logs_cmd,
        submit_user_report
    ]);

    builder
        .setup(|app| {
            let open = MenuItem::with_id(app, "open", "Åbn", true, None::<&str>)?;
            let hide = MenuItem::with_id(app, "hide", "Skjul", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Afslut", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &hide, &quit])?;

            let icon = app
                .default_window_icon()
                .ok_or_else(|| "Standardikon mangler under opstart".to_string())?
                .clone();

            let version = get_app_version();
            let tooltip = format!("Eira Companion v{version}");

            let _tray = TrayIconBuilder::with_id("aimultichat-tray")
                .icon(icon)
                .tooltip(tooltip)
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "hide" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        toggle_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            // Close-to-tray implementation
            if let Some(window) = app.get_webview_window("main") {
                let win_clone = window.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Err(e) = win_clone.hide() {
                            eprintln!("Fejl ved luk til tray: {e}");
                        }
                    }
                });
            }

            // Soft-fail global shortcut registration
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            {
                use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};
                let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyE);
                if let Err(e) = app.global_shortcut().register(shortcut) {
                    eprintln!("Advarsel: Kunne ikke registrere global genvej Ctrl+Shift+E: {e}");
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
