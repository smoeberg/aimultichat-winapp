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
    pub reporter: Mutex<ErrorReporter>,
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

    let mut reporter = state.reporter.lock().map_err(|e| e.to_string())?;
    reporter.update_server_url(&url);
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
    let (server_url, device_id, min_interval, last_report_time_val) = {
        let reporter = state.reporter.lock().map_err(|e| e.to_string())?;
        (
            reporter.server_url.clone(),
            reporter.device_id.clone(),
            reporter.min_interval,
            *reporter.last_report_time.lock().unwrap(),
        )
    };

    if last_report_time_val.elapsed() < min_interval {
        return Err("Rate limited (max 1 report per minute)".to_string());
    }

    {
        if let Ok(mut reporter) = state.reporter.lock() {
            *reporter.last_report_time.lock().unwrap() = std::time::Instant::now();
        }
    }

    let report = error_reporter::ErrorReport {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        os_version: std::env::consts::OS.to_string(),
        error_type: error_type.clone(),
        error_message: error_message.clone(),
        stack_trace: None,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        device_id,
        request_id: None,
        context,
    };

    let client = reqwest::Client::new();
    let response_result = client
        .post(format!("{}/api/companion/error", server_url))
        .json(&report)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await;

    match response_result {
        Ok(resp) => {
            if resp.status().is_success() {
                if let Ok(data) = resp.json::<error_reporter::ReportResponse>().await {
                    Ok(serde_json::json!({
                        "success": true,
                        "report_id": data.id,
                    }))
                } else {
                    Ok(serde_json::json!({ "success": true }))
                }
            } else {
                let err_str = format!("Server svarede med status: {}", resp.status());
                if let Ok(mut logger) = state.logger.lock() {
                    logger.log("error", "error_reporter", &err_str, None);
                }
                Err(err_str)
            }
        }
        Err(e) => {
            let err_str = format!("Kunne ikke sende fejlrapport til server: {}", e);
            if let Ok(mut logger) = state.logger.lock() {
                logger.log("error", "error_reporter", &err_str, None);
            }
            Err(err_str)
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

    let (server_url, device_id) = {
        let reporter = state.reporter.lock().map_err(|e| e.to_string())?;
        (reporter.server_url.clone(), reporter.device_id.clone())
    };

    let report = error_reporter::ErrorReport {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        os_version: std::env::consts::OS.to_string(),
        error_type: "user_report".to_string(),
        error_message: "Bruger har indsendt en fejlrapport".to_string(),
        stack_trace: None,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        device_id,
        request_id: None,
        context: Some(context),
    };

    let client = reqwest::Client::new();
    let response_result = client
        .post(format!("{}/api/companion/error", server_url))
        .json(&report)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await;

    match response_result {
        Ok(resp) => {
            if resp.status().is_success() {
                let report_id = resp.json::<error_reporter::ReportResponse>()
                    .await
                    .map(|r| r.id)
                    .unwrap_or_else(|_| "unknown".to_string());
                Ok(serde_json::json!({
                    "success": true,
                    "report_id": report_id,
                    "message": "Tak for din fejlrapport. Den er sendt til support.",
                }))
            } else {
                Err(format!("Server svarede med status: {}", resp.status()))
            }
        }
        Err(e) => Err(format!("Kunne ikke sende fejlrapport: {}", e)),
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
        reporter: Mutex::new(reporter),
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
