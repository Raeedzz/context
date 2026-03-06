use crate::state::{ActivityRecord, ClickableItem, OverlayContent, WindowInfo};
use std::collections::VecDeque;
use std::time::Instant;

pub fn categorize(window: &mut WindowInfo) {
    let app_lower = window.app_name.to_lowercase();
    let title_lower = window.title.to_lowercase();

    if is_terminal(&app_lower) {
        if title_lower.contains("claude") {
            window.display_text = describe_claude_terminal(&window.title);
        } else if has_test_keywords(&title_lower) {
            window.display_text = describe_test_terminal(&title_lower, &window.title);
        } else {
            window.display_text = describe_terminal(&window.title, &window.app_name);
        }
    } else if is_editor(&app_lower) {
        window.display_text = describe_editor(&window.title, &window.app_name);
    } else if is_browser(&app_lower) {
        window.display_text = describe_browser(&window.title, &window.app_name);
    } else if is_chat(&app_lower) {
        window.display_text = describe_chat(&window.title, &window.app_name);
    } else {
        window.display_text = describe_generic(&window.title, &window.app_name);
    }
}

pub fn build_content(
    windows: &[WindowInfo],
    history: &VecDeque<ActivityRecord>,
) -> OverlayContent {
    let now = Instant::now();

    // Current window items
    let mut items: Vec<ClickableItem> = windows
        .iter()
        .map(|w| ClickableItem {
            id: w.id.clone(),
            label: w.display_text.clone(),
            app_name: w.app_name.clone(),
            is_stale: false,
        })
        .collect();

    // Add recently-gone items (seen in last 5 min but not currently open)
    let current_texts: Vec<&str> = windows.iter().map(|w| w.display_text.as_str()).collect();
    for record in history {
        if !current_texts.contains(&record.display_text.as_str()) {
            let age = now.duration_since(record.last_seen);
            if age.as_secs() < 300 {
                items.push(ClickableItem {
                    id: format!("stale:{}", record.display_text),
                    label: record.display_text.clone(),
                    app_name: record.app_name.clone(),
                    is_stale: true,
                });
            }
        }
    }

    let empty = items.is_empty();

    OverlayContent {
        markdown: if empty {
            "No windows detected. Grant accessibility permissions in System Settings > Privacy & Security > Accessibility.".to_string()
        } else {
            String::new()
        },
        items,
        gemini_summary: None,
    }
}

fn is_terminal(app: &str) -> bool {
    matches!(
        app,
        "terminal" | "iterm2" | "iterm" | "alacritty" | "kitty" | "warp" | "hyper" | "wezterm" | "ghostty"
    )
}

fn is_editor(app: &str) -> bool {
    app.contains("code")
        || app.contains("cursor")
        || app.contains("sublime")
        || app.contains("atom")
        || app.contains("intellij")
        || app.contains("webstorm")
        || app.contains("pycharm")
        || app.contains("vim")
        || app.contains("neovim")
        || app.contains("emacs")
        || app.contains("zed")
        || app.contains("xcode")
        || app.contains("android studio")
        || app.contains("nova")
        || app.contains("fleet")
        || app.contains("textmate")
        || app.contains("bbedit")
        || app.contains("coteditor")
        || app.contains("lapce")
        || app.contains("helix")
        || app.contains("lite-xl")
        || app.contains("micro")
        || app.contains("notepad")
        || app.contains("windsurf")
        || app.contains("trae")
}

fn is_browser(app: &str) -> bool {
    matches!(
        app,
        "safari"
            | "google chrome"
            | "firefox"
            | "microsoft edge"
            | "brave browser"
            | "arc"
            | "opera"
            | "vivaldi"
            | "chromium"
            | "orion"
            | "zen"
            | "zen browser"
    )
}

fn is_chat(app: &str) -> bool {
    matches!(
        app,
        "slack" | "discord" | "telegram" | "messages" | "whatsapp" | "microsoft teams" | "zoom" | "signal"
    )
}

fn has_test_keywords(title: &str) -> bool {
    title.contains("npm test")
        || title.contains("cargo test")
        || title.contains("pytest")
        || title.contains("jest")
        || title.contains("vitest")
        || title.contains("mocha")
}

fn describe_claude_terminal(title: &str) -> String {
    let cleaned = title
        .replace("claude", "")
        .replace("Claude", "")
        .trim_matches(|c: char| c == '-' || c == ':' || c == ' ')
        .to_string();

    if cleaned.is_empty() {
        "Claude Code is running".to_string()
    } else {
        format!("Claude Code is working on {}", truncate(&cleaned, 40))
    }
}

fn describe_test_terminal(title_lower: &str, title: &str) -> String {
    let project = extract_project_from_path(title);
    if title_lower.contains("pass") {
        format!("Tests passing in {}", project)
    } else if title_lower.contains("fail") {
        format!("Tests failing in {}", project)
    } else {
        format!("Running tests in {}", project)
    }
}

fn describe_terminal(title: &str, app: &str) -> String {
    let title_lower = title.to_lowercase();

    if title_lower.contains("git push") || title_lower.contains("git commit") {
        let project = extract_project_from_path(title);
        return format!("Pushing code in {}", project);
    }
    if title_lower.contains("git pull") || title_lower.contains("git fetch") {
        let project = extract_project_from_path(title);
        return format!("Pulling changes in {}", project);
    }
    if title_lower.contains("npm install")
        || title_lower.contains("yarn add")
        || title_lower.contains("pnpm add")
    {
        return "Installing dependencies".to_string();
    }
    if title_lower.contains("npm run dev")
        || title_lower.contains("npm start")
        || title_lower.contains("cargo run")
    {
        let project = extract_project_from_path(title);
        return format!("{} dev server is running", project);
    }
    if title_lower.contains("ssh ") {
        return format!("{} remote session", app);
    }
    if title_lower.contains("docker") {
        return "Docker is running".to_string();
    }

    let project = extract_project_from_path(title);
    format!("{} terminal in {}", app, project)
}

fn describe_editor(title: &str, app: &str) -> String {
    let editor = extract_editor_name(app);
    let parts: Vec<&str> = title.split(" - ").collect();

    if parts.len() >= 2 {
        let file = parts[0].trim();
        let project = parts[1].trim();
        let title_lower = title.to_lowercase();
        if title_lower.contains("git") || title_lower.contains("source control") {
            return format!("Push code in {}", editor);
        }
        format!("Editing {} in {} ({})", file, project, editor)
    } else {
        format!("Working in {}", editor)
    }
}

fn describe_browser(title: &str, app: &str) -> String {
    let clean = strip_browser_suffix(title);
    let clean_lower = clean.to_lowercase();
    let browser = extract_browser_name(app);

    if clean_lower.contains("claude") || clean_lower.contains("anthropic") {
        return format!("Claude is helping you on {}", browser);
    }
    if clean_lower.contains("chatgpt") || clean_lower.contains("openai") {
        return format!("ChatGPT is helping you on {}", browser);
    }
    if clean_lower.contains("github") {
        if clean_lower.contains("pull request") || clean_lower.contains("pr #") {
            return format!("Reviewing a PR on GitHub ({})", browser);
        }
        if clean_lower.contains("issues") {
            return format!("Checking issues on GitHub ({})", browser);
        }
        if clean_lower.contains("actions") {
            return format!("Checking CI on GitHub ({})", browser);
        }
        return format!("Browsing GitHub on {}", browser);
    }
    if clean_lower.contains("stackoverflow") || clean_lower.contains("stack overflow") {
        return format!("Looking up answers on Stack Overflow ({})", browser);
    }
    if clean_lower.contains("youtube") {
        return format!("Watching YouTube on {}", browser);
    }
    if clean_lower.contains("google")
        && (clean_lower.contains("search") || clean.len() < 30)
    {
        return format!("Searching on Google ({})", browser);
    }
    if clean_lower.contains("docs")
        || clean_lower.contains("documentation")
        || clean_lower.contains("readme")
        || clean_lower.contains("api reference")
    {
        return format!("Reading docs: {} ({})", truncate(&clean, 30), browser);
    }
    if clean_lower.contains("linkedin") {
        return format!("LinkedIn is open on {}", browser);
    }
    if clean_lower.contains("twitter") || clean_lower.contains("x.com") {
        return format!("Twitter/X is open on {}", browser);
    }
    if clean_lower.contains("reddit") {
        return format!("Browsing Reddit on {}", browser);
    }
    if clean_lower.contains("canvas")
        || clean_lower.contains("blackboard")
        || clean_lower.contains("gradescope")
        || clean_lower.contains("coursera")
        || clean_lower.contains("piazza")
        || clean_lower.contains("chegg")
    {
        return format!("Doing homework: {} ({})", truncate(&clean, 30), browser);
    }
    if clean_lower.contains("notion") {
        return format!("Working in Notion on {}", browser);
    }
    if clean_lower.contains("figma") {
        return format!("Designing in Figma on {}", browser);
    }
    if clean_lower.contains("gmail") || clean_lower.contains("mail") {
        return format!("Checking email on {}", browser);
    }

    format!("{} ({})", truncate(&clean, 45), browser)
}

fn describe_chat(title: &str, app: &str) -> String {
    let app_cap = capitalize(app);
    if title.is_empty() || title == app {
        format!("{} is open", app_cap)
    } else {
        format!("Chatting on {} - {}", app_cap, truncate(title, 35))
    }
}

fn describe_generic(title: &str, app: &str) -> String {
    if title.is_empty() || title == app {
        format!("{} is open", app)
    } else {
        format!("{} in {}", truncate(title, 40), app)
    }
}

fn extract_project_from_path(title: &str) -> String {
    let cleaned = title
        .split(&['\u{2014}', '|', '-'][..])
        .next()
        .unwrap_or(title)
        .trim();

    if let Some(last) = cleaned.split('/').filter(|s| !s.is_empty()).last() {
        return last.trim().to_string();
    }
    if let Some(last) = cleaned.split(':').last() {
        let p = last.trim().trim_start_matches("~/");
        if let Some(proj) = p.split('/').filter(|s| !s.is_empty()).last() {
            return proj.to_string();
        }
    }
    truncate(cleaned, 30)
}

fn extract_editor_name(app: &str) -> String {
    let app_lower = app.to_lowercase();
    if app_lower.contains("cursor") {
        "Cursor".to_string()
    } else if app_lower.contains("visual studio code") || app_lower == "code" {
        "VS Code".to_string()
    } else if app_lower.contains("windsurf") {
        "Windsurf".to_string()
    } else if app_lower.contains("trae") {
        "Trae".to_string()
    } else if app_lower.contains("zed") {
        "Zed".to_string()
    } else if app_lower.contains("xcode") {
        "Xcode".to_string()
    } else if app_lower.contains("sublime") {
        "Sublime".to_string()
    } else if app_lower.contains("intellij") {
        "IntelliJ".to_string()
    } else {
        capitalize(app)
    }
}

fn extract_browser_name(app: &str) -> String {
    let app_lower = app.to_lowercase();
    if app_lower.contains("chrome") {
        "Chrome".to_string()
    } else if app_lower == "arc" {
        "Arc".to_string()
    } else if app_lower.contains("safari") {
        "Safari".to_string()
    } else if app_lower.contains("firefox") {
        "Firefox".to_string()
    } else if app_lower.contains("brave") {
        "Brave".to_string()
    } else if app_lower.contains("edge") {
        "Edge".to_string()
    } else {
        capitalize(app)
    }
}

fn strip_browser_suffix(title: &str) -> String {
    let suffixes = [
        " - Google Chrome",
        " - Safari",
        " - Firefox",
        " - Microsoft Edge",
        " - Brave",
        " - Arc",
        " - Opera",
        " - Vivaldi",
        " \u{2014} Mozilla Firefox",
    ];
    let mut clean = title.to_string();
    for suffix in &suffixes {
        if let Some(stripped) = clean.strip_suffix(suffix) {
            clean = stripped.to_string();
            break;
        }
    }
    clean
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}
