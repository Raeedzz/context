use crate::state::WindowInfo;
use std::process::Command;

pub fn enumerate_windows() -> Vec<WindowInfo> {
    #[cfg(target_os = "macos")]
    {
        enumerate_macos()
    }
    #[cfg(target_os = "windows")]
    {
        enumerate_windows_os()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Vec::new()
    }
}

#[cfg(target_os = "macos")]
fn enumerate_macos() -> Vec<WindowInfo> {
    let script = r#"
        set output to ""
        tell application "System Events"
            set allProcesses to every process whose visible is true
            repeat with proc in allProcesses
                set procName to name of proc
                try
                    set allWindows to every window of proc
                    repeat with w in allWindows
                        set winTitle to name of w
                        if winTitle is not "" then
                            set output to output & procName & " ||| " & winTitle & linefeed
                        end if
                    end repeat
                end try
            end repeat
        end tell
        return output
    "#;

    let result = Command::new("osascript").arg("-e").arg(script).output();

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut windows = Vec::new();
            let mut id_counter = 0;

            for line in stdout.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Some((app, title)) = line.split_once(" ||| ") {
                    let app = app.trim().to_string();
                    let title = title.trim().to_string();

                    // Filter out the Context overlay itself
                    if app == "Context" {
                        continue;
                    }

                    let id = format!("{}:{}", app, id_counter);
                    id_counter += 1;

                    windows.push(WindowInfo {
                        id,
                        app_name: app,
                        title,
                        category: String::new(),
                        display_text: String::new(),
                    });
                }
            }
            windows
        }
        Err(_) => Vec::new(),
    }
}
