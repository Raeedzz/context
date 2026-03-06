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
    let script = format!(
        r#"
        tell application "{}" to activate
        "#,
        app_name.replace('"', "\\\"")
    );
    let _ = Command::new("osascript").arg("-e").arg(&script).output();
}
