use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, Window,
    command,
};

#[command]
fn get_chat_url() -> String {
    // Kan overstyres via miljøvariabel ved kompilering ellers standard Eira URL
    std::env::var("EIRA_CHAT_URL").unwrap_or_else(|_| "https://ai.eira.dk/chat?embed=companion".into())
}

fn toggle_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        // Monitor boundary check: sikker at vinduet ikke er off-screen
        if let Ok(Some(monitor)) = window.current_monitor().map(|m| m.or_else(|| window.primary_monitor().ok().flatten())) {
            let monitor_size = monitor.size();
            if let Ok(pos) = window.outer_position() {
                if pos.x < 0 || pos.x > monitor_size.width as i32 || pos.y < 0 || pos.y > monitor_size.height as i32 {
                    let _ = window.center();
                }
            }
        }

        if window.is_visible().unwrap_or(true) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
        )
        .invoke_handler(tauri::generate_handler![get_chat_url])
        .setup(|app| {
            let open = MenuItem::with_id(app, "open", "Åbn", true, None::<&str>)?;
            let hide = MenuItem::with_id(app, "hide", "Skjul", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Afslut", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &hide, &quit])?;

            let _tray = TrayIconBuilder::with_id("aimultichat-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Eira Companion")
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

            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            {
                use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};
                let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyE);
                app.global_shortcut().register(shortcut)?;
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
