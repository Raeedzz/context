use std::process::Command;

pub fn focus_window(app_name: &str) {
    #[cfg(target_os = "macos")]
    {
        focus_macos(app_name);
    }
    #[cfg(target_os = "windows")]
    {
        let _ = app_name;
    }
}

#[cfg(target_os = "macos")]
fn focus_macos(app_name: &str) {
    let escaped = app_name.replace('"', "\\\"");

    // Use System Events to set frontmost + raise window, then activate the app.
    // This reliably switches Spaces on macOS.
    let script = format!(
        r#"
        tell application "System Events"
            try
                tell process "{name}"
                    set frontmost to true
                    try
                        perform action "AXRaise" of front window
                    end try
                end tell
            end try
        end tell
        tell application "{name}" to activate
        "#,
        name = escaped
    );
    let _ = Command::new("osascript").arg("-e").arg(&script).output();
}
