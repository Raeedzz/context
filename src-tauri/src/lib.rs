mod commands;
mod gemini;
mod heuristics;
mod state;
mod window_enum;
mod window_focus;

use state::SharedState;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shortcut = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Slash);

    tauri::Builder::default()
        .manage(SharedState::default())
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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_overlay_content,
            commands::focus_window,
            commands::dismiss_overlay,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
