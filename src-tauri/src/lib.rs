mod ax_reader;
mod commands;
mod gemini;
mod heuristics;
mod state;
mod window_enum;
mod window_focus;

use state::SharedState;
use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shortcut = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Slash);

    tauri::Builder::default()
        .manage(SharedState::default())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            commands::toggle_overlay(app);
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        commands::toggle_overlay(app);
                    }
                })
                .build(),
        )
        .setup(move |app| {
            app.global_shortcut().register(shortcut)?;

            // Apply NSWindow properties once during setup, before any show/hide.
            // This prevents the async with_webview race condition on toggle.
            #[cfg(target_os = "macos")]
            {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.with_webview(|webview| {
                        use objc2_app_kit::{
                            NSColor, NSScreenSaverWindowLevel, NSWindowCollectionBehavior,
                        };
                        unsafe {
                            let ns_window: *mut objc2_app_kit::NSWindow =
                                webview.ns_window().cast();
                            if !ns_window.is_null() {
                                let ns_win = &*ns_window;
                                // CanJoinAllSpaces + FullScreenAuxiliary = appears over fullscreen apps
                                // IgnoresCycle = doesn't show in Cmd+Tab
                                // NO Stationary — it blocks fullscreen space entry
                                ns_win.setCollectionBehavior(
                                    NSWindowCollectionBehavior::CanJoinAllSpaces
                                        | NSWindowCollectionBehavior::FullScreenAuxiliary
                                        | NSWindowCollectionBehavior::IgnoresCycle,
                                );
                                ns_win.setLevel(NSScreenSaverWindowLevel - 1);
                                ns_win.setBackgroundColor(Some(&NSColor::clearColor()));
                                ns_win.setOpaque(false);
                                ns_win.setHasShadow(false);
                            }
                        }
                    });
                }
            }

            // Auto-show as a widget on startup
            commands::toggle_overlay(app.handle());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_overlay_content,
            commands::focus_window,
            commands::dismiss_overlay,
            commands::toggle_context,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
