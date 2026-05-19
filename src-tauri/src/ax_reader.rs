use std::process::Command;

#[derive(Debug, Clone)]
pub struct BrowserTab {
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct BrowserData {
    pub app_name: String,
    pub tabs: Vec<BrowserTab>,
}

#[derive(Debug, Clone)]
pub struct TerminalData {
    pub app_name: String,
    pub visible_text: String,
}

#[derive(Debug, Clone)]
pub struct ForegroundContent {
    pub app_name: String,
    pub focused_text: String,
    pub selected_text: String,
}

#[derive(Debug, Clone)]
pub struct GitStatus {
    pub repo_name: String,
    pub branch: String,
    pub status_short: String,
    pub last_commit: String,
}

#[derive(Debug, Clone)]
pub struct NotificationData {
    pub app_name: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ShellSession {
    pub terminal_app: String,
    pub cwd: String,
    pub project_name: String,
    pub project_description: String,
    pub running_command: String,
}

#[derive(Debug, Clone)]
pub struct DeepContext {
    pub browsers: Vec<BrowserData>,
    pub terminals: Vec<TerminalData>,
    pub shell_sessions: Vec<ShellSession>,
    pub foreground: Option<ForegroundContent>,
    pub git_repos: Vec<GitStatus>,
    pub notifications: Vec<NotificationData>,
    pub recent_commands: Vec<String>,
}

pub fn read_deep_context(windows: &[crate::state::WindowInfo]) -> DeepContext {
    #[cfg(target_os = "macos")]
    {
        let browsers = read_all_browser_tabs();
        let terminals = read_terminal_output();
        let shell_sessions = read_shell_sessions();
        let foreground = read_foreground_content();
        let git_repos = read_git_status(windows);
        let notifications = read_notifications();
        let recent_commands = read_recent_commands();
        DeepContext {
            browsers,
            terminals,
            shell_sessions,
            foreground,
            git_repos,
            notifications,
            recent_commands,
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = windows;
        DeepContext {
            browsers: Vec::new(),
            terminals: Vec::new(),
            shell_sessions: Vec::new(),
            foreground: None,
            git_repos: Vec::new(),
            notifications: Vec::new(),
            recent_commands: Vec::new(),
        }
    }
}

#[cfg(target_os = "macos")]
fn read_all_browser_tabs() -> Vec<BrowserData> {
    let mut results = Vec::new();

    if let Some(data) = read_chrome_tabs("Google Chrome") {
        results.push(data);
    }
    if let Some(data) = read_chrome_tabs("Arc") {
        results.push(data);
    }
    if let Some(data) = read_chrome_tabs("Brave Browser") {
        results.push(data);
    }
    if let Some(data) = read_chrome_tabs("Microsoft Edge") {
        results.push(data);
    }
    if let Some(data) = read_safari_tabs() {
        results.push(data);
    }

    results
}

#[cfg(target_os = "macos")]
fn read_chrome_tabs(app_name: &str) -> Option<BrowserData> {
    let check_script = format!(
        r#"tell application "System Events" to (name of processes) contains "{}""#,
        app_name
    );
    let check = Command::new("osascript")
        .arg("-e")
        .arg(&check_script)
        .output()
        .ok()?;
    let running = String::from_utf8_lossy(&check.stdout)
        .trim()
        .to_string();
    if running != "true" {
        return None;
    }

    let script = format!(
        r#"
        set output to ""
        tell application "{}"
            repeat with w in windows
                repeat with t in tabs of w
                    set tabTitle to title of t
                    set tabUrl to URL of t
                    set output to output & tabTitle & " ||| " & tabUrl & linefeed
                end repeat
            end repeat
        end tell
        return output
        "#,
        app_name
    );

    let result = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&result.stdout);

    let mut tabs = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((title, url)) = line.split_once(" ||| ") {
            tabs.push(BrowserTab {
                title: title.trim().to_string(),
                url: url.trim().to_string(),
            });
        }
    }

    if tabs.is_empty() {
        return None;
    }

    Some(BrowserData {
        app_name: app_name.to_string(),
        tabs,
    })
}

#[cfg(target_os = "macos")]
fn read_safari_tabs() -> Option<BrowserData> {
    let check_script =
        r#"tell application "System Events" to (name of processes) contains "Safari""#;
    let check = Command::new("osascript")
        .arg("-e")
        .arg(check_script)
        .output()
        .ok()?;
    let running = String::from_utf8_lossy(&check.stdout)
        .trim()
        .to_string();
    if running != "true" {
        return None;
    }

    let script = r#"
        set output to ""
        tell application "Safari"
            repeat with w in windows
                repeat with t in tabs of w
                    set tabTitle to name of t
                    set tabUrl to URL of t
                    set output to output & tabTitle & " ||| " & tabUrl & linefeed
                end repeat
            end repeat
        end tell
        return output
    "#;

    let result = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&result.stdout);

    let mut tabs = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((title, url)) = line.split_once(" ||| ") {
            tabs.push(BrowserTab {
                title: title.trim().to_string(),
                url: url.trim().to_string(),
            });
        }
    }

    if tabs.is_empty() {
        return None;
    }

    Some(BrowserData {
        app_name: "Safari".to_string(),
        tabs,
    })
}

#[cfg(target_os = "macos")]
fn read_terminal_output() -> Vec<TerminalData> {
    let mut results = Vec::new();

    if let Some(data) = read_terminal_app_content() {
        results.push(data);
    }
    if let Some(data) = read_iterm_content() {
        results.push(data);
    }
    for app in &["Ghostty", "kitty", "Alacritty", "Warp"] {
        if let Some(data) = read_ax_terminal_content(app) {
            results.push(data);
        }
    }

    results
}

#[cfg(target_os = "macos")]
fn read_ax_terminal_content(app_name: &str) -> Option<TerminalData> {
    let check_script = format!(
        r#"tell application "System Events" to (name of processes) contains "{}""#,
        app_name
    );
    let check = Command::new("osascript")
        .arg("-e")
        .arg(&check_script)
        .output()
        .ok()?;
    if String::from_utf8_lossy(&check.stdout).trim() != "true" {
        return None;
    }

    let script = format!(
        r#"
        tell application "System Events"
            tell process "{}"
                try
                    set frontWindow to front window
                    set allText to ""
                    repeat with uiElem in entire contents of frontWindow
                        try
                            set elemRole to role of uiElem
                            if elemRole is "AXTextArea" or elemRole is "AXStaticText" then
                                set elemValue to value of uiElem
                                if elemValue is not missing value and elemValue is not "" then
                                    set allText to allText & elemValue & linefeed
                                end if
                            end if
                        end try
                    end repeat
                    if allText is "" then
                        try
                            set focusedElem to focused UI element of frontWindow
                            set allText to value of focusedElem
                        end try
                    end if
                    set lineList to paragraphs of allText
                    set lineCount to count of lineList
                    set startLine to lineCount - 50
                    if startLine < 1 then set startLine to 1
                    set output to ""
                    repeat with i from startLine to lineCount
                        set output to output & (item i of lineList) & linefeed
                    end repeat
                    return output
                on error
                    return ""
                end try
            end tell
        end tell
        "#,
        app_name
    );

    let result = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&result.stdout).trim().to_string();

    if text.is_empty() {
        return None;
    }

    Some(TerminalData {
        app_name: app_name.to_string(),
        visible_text: text,
    })
}

#[cfg(target_os = "macos")]
fn read_terminal_app_content() -> Option<TerminalData> {
    let check_script =
        r#"tell application "System Events" to (name of processes) contains "Terminal""#;
    let check = Command::new("osascript")
        .arg("-e")
        .arg(check_script)
        .output()
        .ok()?;
    if String::from_utf8_lossy(&check.stdout).trim() != "true" {
        return None;
    }

    let script = r#"
        tell application "Terminal"
            if (count of windows) > 0 then
                set visibleContent to contents of selected tab of front window
                set lineList to paragraphs of visibleContent
                set lineCount to count of lineList
                set startLine to lineCount - 30
                if startLine < 1 then set startLine to 1
                set output to ""
                repeat with i from startLine to lineCount
                    set output to output & (item i of lineList) & linefeed
                end repeat
                return output
            end if
        end tell
        return ""
    "#;

    let result = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&result.stdout).trim().to_string();

    if text.is_empty() {
        return None;
    }

    Some(TerminalData {
        app_name: "Terminal".to_string(),
        visible_text: text,
    })
}

#[cfg(target_os = "macos")]
fn read_iterm_content() -> Option<TerminalData> {
    let check_script =
        r#"tell application "System Events" to (name of processes) contains "iTerm2""#;
    let check = Command::new("osascript")
        .arg("-e")
        .arg(check_script)
        .output()
        .ok()?;
    if String::from_utf8_lossy(&check.stdout).trim() != "true" {
        return None;
    }

    let script = r#"
        tell application "iTerm2"
            if (count of windows) > 0 then
                tell current session of current tab of current window
                    set visibleContent to contents
                    set lineList to paragraphs of visibleContent
                    set lineCount to count of lineList
                    set startLine to lineCount - 50
                    if startLine < 1 then set startLine to 1
                    set output to ""
                    repeat with i from startLine to lineCount
                        set output to output & (item i of lineList) & linefeed
                    end repeat
                    return output
                end tell
            end if
        end tell
        return ""
    "#;

    let result = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&result.stdout).trim().to_string();

    if text.is_empty() {
        return None;
    }

    Some(TerminalData {
        app_name: "iTerm2".to_string(),
        visible_text: text,
    })
}

#[cfg(target_os = "macos")]
fn read_foreground_content() -> Option<ForegroundContent> {
    let script = r#"
        tell application "System Events"
            set frontApp to name of first application process whose frontmost is true
            if frontApp is "Context" then return ""
            if frontApp is "Finder" then return ""
            if frontApp is "Dock" then return ""

            set appName to frontApp
            set focusedText to ""
            set selectedText to ""

            try
                tell process frontApp
                    try
                        set focusedElem to focused UI element of front window
                        try
                            set focusedText to value of focusedElem
                        end try
                        try
                            set selectedText to value of attribute "AXSelectedText" of focusedElem
                        end try
                    end try

                    if focusedText is "" or focusedText is missing value then
                        set focusedText to ""
                        try
                            repeat with uiElem in entire contents of front window
                                try
                                    set elemRole to role of uiElem
                                    if elemRole is "AXTextArea" then
                                        set elemValue to value of uiElem
                                        if elemValue is not missing value and elemValue is not "" then
                                            set lineList to paragraphs of elemValue
                                            set lineCount to count of lineList
                                            set startLine to lineCount - 30
                                            if startLine < 1 then set startLine to 1
                                            set output to ""
                                            repeat with i from startLine to lineCount
                                                set output to output & (item i of lineList) & linefeed
                                            end repeat
                                            set focusedText to output
                                            exit repeat
                                        end if
                                    end if
                                end try
                            end repeat
                        end try
                    else
                        if focusedText is not missing value then
                            set lineList to paragraphs of focusedText
                            set lineCount to count of lineList
                            set startLine to lineCount - 30
                            if startLine < 1 then set startLine to 1
                            set output to ""
                            repeat with i from startLine to lineCount
                                set output to output & (item i of lineList) & linefeed
                            end repeat
                            set focusedText to output
                        end if
                    end if
                end tell
            end try

            if focusedText is missing value then set focusedText to ""
            if selectedText is missing value then set selectedText to ""

            return appName & " |||SPLIT||| " & focusedText & " |||SPLIT||| " & selectedText
        end tell
    "#;

    let result = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&result.stdout).trim().to_string();

    if stdout.is_empty() {
        return None;
    }

    let parts: Vec<&str> = stdout.splitn(3, " |||SPLIT||| ").collect();
    if parts.is_empty() {
        return None;
    }

    let app_name = parts.first().unwrap_or(&"").trim().to_string();
    let focused_text = parts.get(1).unwrap_or(&"").trim().to_string();
    let selected_text = parts.get(2).unwrap_or(&"").trim().to_string();

    if app_name.is_empty() || (focused_text.is_empty() && selected_text.is_empty()) {
        return None;
    }

    Some(ForegroundContent {
        app_name,
        focused_text,
        selected_text,
    })
}

// --- Shell sessions (process-based terminal reading) ---

#[cfg(target_os = "macos")]
fn read_shell_sessions() -> Vec<ShellSession> {
    let output = match Command::new("ps")
        .args(["-eo", "pid,ppid,comm"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse process list (pid, ppid, comm)
    let mut processes: Vec<(u32, u32, String)> = Vec::new();
    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            if let (Ok(pid), Ok(ppid)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                let comm = parts[2..].join(" ");
                processes.push((pid, ppid, comm));
            }
        }
    }

    // Get full args for all processes (for richer child process info)
    let args_output = Command::new("ps")
        .args(["-eo", "pid,args"])
        .output()
        .ok();
    let mut args_map: std::collections::HashMap<u32, String> = std::collections::HashMap::new();
    if let Some(ao) = args_output {
        let args_text = String::from_utf8_lossy(&ao.stdout);
        for line in args_text.lines().skip(1) {
            let trimmed = line.trim_start();
            if let Some(space_idx) = trimmed.find(' ') {
                if let Ok(pid) = trimmed[..space_idx].parse::<u32>() {
                    let args = trimmed[space_idx..].trim().to_string();
                    args_map.insert(pid, args);
                }
            }
        }
    }

    // Find shell processes
    let shells: Vec<(u32, u32)> = processes
        .iter()
        .filter(|(_, _, comm)| {
            let c = comm.to_lowercase();
            c.contains("zsh") || c.contains("bash") || c.contains("fish")
        })
        .map(|(pid, ppid, _)| (*pid, *ppid))
        .collect();

    // Known terminal app identifiers (comm substring -> display name)
    let terminal_ids: &[(&str, &str)] = &[
        ("stable", "Warp"),
        ("warp", "Warp"),
        ("terminal", "Terminal"),
        ("iterm", "iTerm2"),
        ("ghostty", "Ghostty"),
        ("kitty", "kitty"),
        ("alacritty", "Alacritty"),
        ("wezterm", "WezTerm"),
        ("cursor", "Cursor"),
        ("code", "VS Code"),
    ];

    // Map each shell to its terminal app by walking up PPID chain
    let mut shell_apps: Vec<(u32, String)> = Vec::new();
    for (shell_pid, shell_ppid) in &shells {
        let mut current_ppid = *shell_ppid;
        let mut found_app = String::new();

        for _ in 0..10 {
            if let Some((_, next_ppid, comm)) =
                processes.iter().find(|(pid, _, _)| *pid == current_ppid)
            {
                let comm_lower = comm.to_lowercase();
                // Skip self-references (shell -> shell)
                if comm_lower.contains("zsh")
                    || comm_lower.contains("bash")
                    || comm_lower.contains("fish")
                {
                    current_ppid = *next_ppid;
                    if current_ppid <= 1 {
                        break;
                    }
                    continue;
                }
                for (id, name) in terminal_ids {
                    if comm_lower.contains(id) {
                        found_app = name.to_string();
                        break;
                    }
                }
                if !found_app.is_empty() {
                    break;
                }
                current_ppid = *next_ppid;
                if current_ppid <= 1 {
                    break;
                }
            } else {
                break;
            }
        }

        if !found_app.is_empty() {
            shell_apps.push((*shell_pid, found_app));
        }
    }

    if shell_apps.is_empty() {
        return Vec::new();
    }

    // Batch-get CWDs via lsof
    let pid_list: String = shell_apps
        .iter()
        .map(|(pid, _)| pid.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let lsof_output = Command::new("lsof")
        .args(["-a", "-d", "cwd", "-Fn", "-p", &pid_list])
        .output()
        .ok();

    let mut cwd_map: std::collections::HashMap<u32, String> = std::collections::HashMap::new();
    if let Some(lsof) = lsof_output {
        let text = String::from_utf8_lossy(&lsof.stdout);
        let mut current_pid: Option<u32> = None;
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix('p') {
                current_pid = rest.parse().ok();
            } else if let Some(rest) = line.strip_prefix('n') {
                if let Some(pid) = current_pid {
                    cwd_map.insert(pid, rest.to_string());
                }
            }
        }
    }

    // Build sessions with child process info
    let mut sessions: Vec<ShellSession> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for (shell_pid, terminal_app) in &shell_apps {
        let cwd = match cwd_map.get(shell_pid) {
            Some(c) if !c.is_empty() => c.clone(),
            _ => continue,
        };

        // Find child processes of this shell, using full args
        let children: Vec<String> = processes
            .iter()
            .filter(|(_, ppid, _)| *ppid == *shell_pid)
            .map(|(child_pid, _, comm)| {
                // Use full args if available, otherwise fall back to comm name
                if let Some(full_args) = args_map.get(child_pid) {
                    // Clean up the args: remove full path prefix
                    let cleaned = if full_args.starts_with('/') {
                        // e.g. "/usr/local/bin/node bun run dev" -> "bun run dev"
                        // Find the command name from comm and use args after it
                        let cmd_name = comm.split('/').last().unwrap_or(comm);
                        if let Some(idx) = full_args.find(cmd_name) {
                            full_args[idx..].to_string()
                        } else {
                            full_args.clone()
                        }
                    } else {
                        full_args.clone()
                    };
                    cleaned
                } else {
                    comm.split('/').last().unwrap_or(comm).to_string()
                }
            })
            .collect();

        let running = children.join(", ");

        // Skip our own app
        if running.contains("Context") {
            continue;
        }

        // Deduplicate by CWD + running command
        let key = format!("{}:{}:{}", terminal_app, cwd, running);
        if !seen.insert(key) {
            continue;
        }

        let project_name = std::path::Path::new(&cwd)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| cwd.clone());

        let project_description = read_project_description(&cwd);

        sessions.push(ShellSession {
            terminal_app: terminal_app.clone(),
            cwd,
            project_name,
            project_description,
            running_command: running,
        });
    }

    sessions
}

fn read_project_description(cwd: &str) -> String {
    let cwd_path = std::path::Path::new(cwd);

    // Try package.json
    let pkg_path = cwd_path.join("package.json");
    if let Ok(content) = std::fs::read_to_string(&pkg_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            let name = json["name"].as_str().unwrap_or("");
            let desc = json["description"].as_str().unwrap_or("");
            let scripts: Vec<String> = json["scripts"]
                .as_object()
                .map(|s| s.keys().cloned().collect())
                .unwrap_or_default();
            let mut parts = Vec::new();
            if !name.is_empty() {
                parts.push(name.to_string());
            }
            if !desc.is_empty() {
                parts.push(desc.to_string());
            }
            if !scripts.is_empty() {
                parts.push(format!("scripts: {}", scripts.join(", ")));
            }
            if !parts.is_empty() {
                return parts.join(" | ");
            }
        }
    }

    // Try Cargo.toml
    let cargo_path = cwd_path.join("Cargo.toml");
    let cargo_sub = cwd_path.join("src-tauri/Cargo.toml");
    let cargo_file = if cargo_path.exists() {
        Some(cargo_path)
    } else if cargo_sub.exists() {
        Some(cargo_sub)
    } else {
        None
    };
    if let Some(path) = cargo_file {
        if let Ok(content) = std::fs::read_to_string(&path) {
            let mut name = String::new();
            let mut desc = String::new();
            for line in content.lines() {
                if line.starts_with("name") {
                    if let Some(n) = line.split('"').nth(1) {
                        name = n.to_string();
                    }
                }
                if line.starts_with("description") {
                    if let Some(d) = line.split('"').nth(1) {
                        desc = d.to_string();
                    }
                }
            }
            if !name.is_empty() {
                if !desc.is_empty() {
                    return format!("{} | {}", name, desc);
                }
                return name;
            }
        }
    }

    String::new()
}

// --- Git status ---

#[cfg(target_os = "macos")]
fn read_git_status(windows: &[crate::state::WindowInfo]) -> Vec<GitStatus> {
    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => return Vec::new(),
    };

    let mut repo_paths: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Extract project names from window titles
    for w in windows {
        // Editor/terminal titles: "file - project - Editor" or "user: ~/path"
        for part in w.title.split(" \u{2014} ").chain(w.title.split(" - ")) {
            let candidate = part.trim();
            if candidate.is_empty() || candidate.len() > 60 {
                continue;
            }
            // Skip things that look like file names or common suffixes
            if candidate.contains('.') && !candidate.contains('/') {
                continue;
            }
            for parent in &["Developer", "Projects", "Code", "repos", "src", "work", "Desktop", ""] {
                let path = if parent.is_empty() {
                    format!("{}/{}", home, candidate)
                } else {
                    format!("{}/{}/{}", home, parent, candidate)
                };
                if seen.insert(path.clone()) && std::path::Path::new(&path).join(".git").exists() {
                    repo_paths.push(path);
                }
            }
        }

        // Extract explicit paths from title (~/path or /Users/...)
        for word in w.title.split_whitespace() {
            let expanded = if word.starts_with("~/") {
                word.replacen("~", &home, 1)
            } else if word.starts_with("/Users/") || word.starts_with("/home/") {
                word.to_string()
            } else {
                continue;
            };
            let mut p = std::path::PathBuf::from(&expanded);
            for _ in 0..5 {
                if p.join(".git").exists() {
                    let s = p.to_string_lossy().to_string();
                    if seen.insert(s.clone()) {
                        repo_paths.push(s);
                    }
                    break;
                }
                if !p.pop() {
                    break;
                }
            }
        }
    }

    repo_paths.truncate(5);
    repo_paths.iter().filter_map(|p| get_git_info(p)).collect()
}

#[cfg(target_os = "macos")]
fn get_git_info(repo_path: &str) -> Option<GitStatus> {
    let branch = Command::new("git")
        .args(["-C", repo_path, "branch", "--show-current"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let status = Command::new("git")
        .args(["-C", repo_path, "status", "--short"])
        .output()
        .ok()
        .map(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            s.lines().take(10).collect::<Vec<_>>().join("\n")
        })
        .unwrap_or_default();

    let last_commit = Command::new("git")
        .args(["-C", repo_path, "log", "--oneline", "-1"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    if branch.is_empty() && status.is_empty() && last_commit.is_empty() {
        return None;
    }

    let repo_name = std::path::Path::new(repo_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| repo_path.to_string());

    Some(GitStatus {
        repo_name,
        branch,
        status_short: status,
        last_commit,
    })
}

// --- Notifications ---

#[cfg(target_os = "macos")]
fn read_notifications() -> Vec<NotificationData> {
    let script = r#"
        set output to ""
        tell application "System Events"
            try
                tell process "NotificationCenter"
                    set notifWindows to windows
                    repeat with w in notifWindows
                        repeat with uiElem in entire contents of w
                            try
                                set elemRole to role of uiElem
                                if elemRole is "AXStaticText" then
                                    set elemValue to value of uiElem
                                    if elemValue is not missing value and elemValue is not "" then
                                        set output to output & elemValue & " ||| "
                                    end if
                                end if
                            end try
                        end repeat
                        set output to output & linefeed
                    end repeat
                end tell
            on error
                return ""
            end try
        end tell
        return output
    "#;

    let result = match Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
    {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&result.stdout).trim().to_string();
    if stdout.is_empty() {
        return Vec::new();
    }

    let mut notifications = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line
            .split(" ||| ")
            .filter(|s| !s.trim().is_empty())
            .collect();
        if parts.len() >= 2 {
            notifications.push(NotificationData {
                app_name: parts[0].trim().to_string(),
                text: parts[1..].join(" - ").trim().to_string(),
            });
        } else if !parts.is_empty() {
            notifications.push(NotificationData {
                app_name: "System".to_string(),
                text: parts.join(" ").trim().to_string(),
            });
        }
    }

    notifications.truncate(5);
    notifications
}

// --- Recent shell commands ---

fn read_recent_commands() -> Vec<String> {
    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => return Vec::new(),
    };

    let history_path = format!("{}/.zsh_history", home);

    // Read last 50 lines to find 3 unique commands
    let output = match Command::new("tail")
        .args(["-n", "50", &history_path])
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    let text = String::from_utf8_lossy(&output.stdout);
    let mut commands: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for line in text.lines().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // zsh history format: ": timestamp:duration;command" or just "command"
        let cmd = if let Some(idx) = line.find(';') {
            &line[idx + 1..]
        } else {
            line
        };

        let cmd = cmd.trim();
        if cmd.is_empty() {
            continue;
        }

        // Filter out sensitive commands (API keys, tokens, passwords)
        let lower = cmd.to_lowercase();
        if lower.contains("api_key")
            || lower.contains("apikey")
            || lower.contains("token=")
            || lower.contains("password")
            || lower.contains("secret")
            || lower.contains("export ")
                && (lower.contains("key") || lower.contains("token") || lower.contains("secret"))
        {
            continue;
        }

        if seen.insert(cmd.to_string()) {
            commands.push(cmd.to_string());
            if commands.len() >= 3 {
                break;
            }
        }
    }

    commands.reverse();
    commands
}
