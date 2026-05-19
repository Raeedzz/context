use crate::ax_reader::{BrowserTab, DeepContext};
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
        window.display_text = describe_browser_from_title(&window.title, &window.app_name);
    } else if is_chat(&app_lower) {
        window.display_text = describe_chat(&window.title, &window.app_name);
    } else {
        window.display_text = describe_generic(&window.title, &window.app_name);
    }
}

pub fn build_content(
    windows: &[WindowInfo],
    deep: &DeepContext,
    history: &VecDeque<ActivityRecord>,
) -> OverlayContent {
    let now = Instant::now();
    let mut items: Vec<ClickableItem> = Vec::new();

    // Non-browser windows
    for w in windows {
        if !is_browser(&w.app_name.to_lowercase()) {
            items.push(ClickableItem {
                id: w.id.clone(),
                label: w.display_text.clone(),
                app_name: w.app_name.clone(),
                source_type: classify_source(&w.app_name),
                is_stale: false,
            });
        }
    }

    // Enrich terminal items with output context
    for t in &deep.terminals {
        let snippet = summarize_terminal_output(&t.visible_text);
        if !snippet.is_empty() {
            for item in &mut items {
                if item.app_name.to_lowercase() == t.app_name.to_lowercase() && !item.is_stale {
                    item.label = format!("{} — {}", item.label, snippet);
                }
            }
        }
    }

    // Browser tabs: group by category, show smart summaries
    for browser in &deep.browsers {
        let browser_name = extract_browser_name(&browser.app_name);
        let grouped = group_tabs(&browser.tabs);

        for group in &grouped {
            let label = match group {
                TabGroup::Single(tab) => {
                    describe_browser_tab(tab, &browser_name)
                }
                TabGroup::Grouped { category, tabs, highlight } => {
                    if tabs.len() == 1 {
                        describe_browser_tab(&tabs[0], &browser_name)
                    } else if let Some(h) = highlight {
                        format!("{}: {} (+{} more) ({})", category, truncate(h, 28), tabs.len() - 1, browser_name)
                    } else {
                        format!("{}: {} tabs ({})", category, tabs.len(), browser_name)
                    }
                }
            };
            items.push(ClickableItem {
                id: format!("tab:{}:{}", browser.app_name, label),
                label,
                app_name: browser.app_name.clone(),
                source_type: "browser".to_string(),
                is_stale: false,
            });
        }
    }

    // Fallback to window titles if no deep browser data
    if deep.browsers.is_empty() {
        for w in windows {
            if is_browser(&w.app_name.to_lowercase()) {
                items.push(ClickableItem {
                    id: w.id.clone(),
                    label: w.display_text.clone(),
                    app_name: w.app_name.clone(),
                    source_type: "browser".to_string(),
                    is_stale: false,
                });
            }
        }
    }

    // Stale items from history
    let current_texts: Vec<String> = items.iter().map(|i| i.label.clone()).collect();
    for record in history {
        if !current_texts.iter().any(|t| t == &record.display_text) {
            let age = now.duration_since(record.last_seen);
            if age.as_secs() < 300 {
                items.push(ClickableItem {
                    id: format!("stale:{}", record.display_text),
                    label: record.display_text.clone(),
                    app_name: record.app_name.clone(),
                    source_type: classify_source(&record.app_name),
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
        context_enabled: false,
    }
}

// --- Tab grouping ---

enum TabGroup<'a> {
    Single(&'a BrowserTab),
    Grouped {
        category: String,
        tabs: Vec<&'a BrowserTab>,
        highlight: Option<String>, // most relevant tab title to show
    },
}

fn classify_tab(tab: &BrowserTab) -> &'static str {
    let url = tab.url.to_lowercase();

    if url.contains("github.com") { return "GitHub"; }
    if url.contains("claude.ai") || url.contains("anthropic.com") { return "AI"; }
    if url.contains("chatgpt.com") || url.contains("chat.openai.com") { return "AI"; }
    if url.contains("stackoverflow.com") || url.contains("stackexchange.com") { return "Research"; }
    if url.contains("google.com/search") { return "Research"; }
    if url.contains("docs.") || url.contains("/docs") || url.contains("developer.") || url.contains("devdocs") || url.contains("mdn") { return "Docs"; }
    if url.contains("localhost") || url.contains("127.0.0.1") { return "Local dev"; }
    if url.contains("canvas") || url.contains("blackboard") || url.contains("gradescope") || url.contains("coursera") || url.contains("piazza") || url.contains("chegg") { return "Homework"; }
    if url.contains("youtube.com") { return "YouTube"; }
    if url.contains("reddit.com") { return "Reddit"; }
    if url.contains("twitter.com") || url.contains("x.com") { return "Twitter/X"; }
    if url.contains("linkedin.com") { return "LinkedIn"; }
    if url.contains("notion.so") || url.contains("notion.site") { return "Notion"; }
    if url.contains("figma.com") { return "Figma"; }
    if url.contains("mail.google.com") || url.contains("outlook") { return "Email"; }
    if url.contains("slack.com") { return "Slack"; }
    if url.contains("discord.com") { return "Discord"; }
    if url.contains("docs.google.com") { return "Google Docs"; }
    if url.contains("drive.google.com") { return "Google Drive"; }
    if url.contains("spotify.com") { return "Spotify"; }
    if url.contains("netflix.com") || url.contains("hulu.com") || url.contains("disneyplus.com") { return "Streaming"; }
    "Other"
}

fn group_tabs(tabs: &[BrowserTab]) -> Vec<TabGroup<'_>> {
    use std::collections::BTreeMap;

    let mut by_category: BTreeMap<&str, Vec<&BrowserTab>> = BTreeMap::new();
    for tab in tabs {
        let cat = classify_tab(tab);
        by_category.entry(cat).or_default().push(tab);
    }

    // Important categories show individually, others group
    let important = ["AI", "GitHub", "Homework", "Local dev", "Docs"];

    let mut result: Vec<TabGroup<'_>> = Vec::new();

    // Important categories: show each tab if <=3, otherwise group with highlight
    for cat in &important {
        if let Some(cat_tabs) = by_category.remove(cat) {
            if cat_tabs.len() <= 2 {
                for tab in cat_tabs {
                    result.push(TabGroup::Single(tab));
                }
            } else {
                let highlight = cat_tabs.first().map(|t| t.title.clone());
                result.push(TabGroup::Grouped {
                    category: cat.to_string(),
                    tabs: cat_tabs,
                    highlight,
                });
            }
        }
    }

    // Other categories: always group
    for (cat, cat_tabs) in by_category {
        if cat_tabs.len() == 1 {
            result.push(TabGroup::Single(cat_tabs[0]));
        } else {
            let highlight = cat_tabs.first().map(|t| t.title.clone());
            result.push(TabGroup::Grouped {
                category: cat.to_string(),
                tabs: cat_tabs,
                highlight,
            });
        }
    }

    result
}

fn describe_browser_tab(tab: &BrowserTab, browser: &str) -> String {
    let url_lower = tab.url.to_lowercase();
    let title = &tab.title;

    if url_lower.contains("github.com") {
        if url_lower.contains("/pull/") {
            return format!("Reviewing PR: {} ({})", truncate(title, 35), browser);
        }
        if url_lower.contains("/issues") {
            return format!("GitHub issue: {} ({})", truncate(title, 35), browser);
        }
        if url_lower.contains("/actions") {
            return format!("Checking CI: {} ({})", truncate(title, 38), browser);
        }
        return format!("GitHub: {} ({})", truncate(title, 38), browser);
    }
    if url_lower.contains("claude.ai") || url_lower.contains("anthropic.com") {
        return format!("Claude: {} ({})", truncate(title, 38), browser);
    }
    if url_lower.contains("chatgpt.com") || url_lower.contains("chat.openai.com") {
        return format!("ChatGPT: {} ({})", truncate(title, 38), browser);
    }
    if url_lower.contains("stackoverflow.com") {
        return format!("SO: {} ({})", truncate(title, 40), browser);
    }
    if url_lower.contains("google.com/search") {
        return format!("Googling: {} ({})", truncate(title, 35), browser);
    }
    if url_lower.contains("localhost") || url_lower.contains("127.0.0.1") {
        let port = extract_port(&tab.url);
        return format!("Local dev :{} — {} ({})", port, truncate(title, 25), browser);
    }
    if url_lower.contains("canvas") || url_lower.contains("blackboard") || url_lower.contains("gradescope") || url_lower.contains("coursera") || url_lower.contains("piazza") || url_lower.contains("chegg") {
        return format!("Homework: {} ({})", truncate(title, 35), browser);
    }
    if url_lower.contains("docs.") || url_lower.contains("/docs") || url_lower.contains("developer.") || url_lower.contains("mdn") {
        return format!("Docs: {} ({})", truncate(title, 38), browser);
    }

    let domain = extract_domain(&tab.url);
    format!("{} [{}] ({})", truncate(title, 28), domain, browser)
}

fn describe_browser_from_title(title: &str, app: &str) -> String {
    let clean = strip_browser_suffix(title);
    let browser = extract_browser_name(app);
    format!("{} ({})", truncate(&clean, 45), browser)
}

fn summarize_terminal_output(text: &str) -> String {
    let lines: Vec<&str> = text.lines().rev().take(15).collect();
    let text_lower = lines.join("\n").to_lowercase();

    if text_lower.contains("error") || text_lower.contains("failed") || text_lower.contains("panic") {
        for line in &lines {
            let ll = line.to_lowercase();
            if ll.contains("error") || ll.contains("failed") || ll.contains("panic") {
                let clean = line.trim();
                if !clean.is_empty() && clean.len() > 5 {
                    return format!("err: {}", truncate(clean, 45));
                }
            }
        }
        return "errors detected".to_string();
    }
    if text_lower.contains("warning") || text_lower.contains("warn") {
        return "warnings in output".to_string();
    }
    if text_lower.contains("pass") && text_lower.contains("test") {
        return "tests passing".to_string();
    }
    if text_lower.contains("compiling") || text_lower.contains("building") {
        return "building...".to_string();
    }
    if text_lower.contains("watching") || text_lower.contains("ready") || text_lower.contains("listening") {
        return "ready".to_string();
    }
    if text_lower.contains("installing") || text_lower.contains("downloading") {
        return "installing...".to_string();
    }

    String::new()
}

fn extract_domain(url: &str) -> String {
    url.replace("https://", "")
        .replace("http://", "")
        .split('/')
        .next()
        .unwrap_or("")
        .replace("www.", "")
        .to_string()
}

fn extract_port(url: &str) -> String {
    let stripped = url.replace("https://", "").replace("http://", "");
    if let Some(host_port) = stripped.split('/').next() {
        if let Some(port) = host_port.split(':').nth(1) {
            return port.to_string();
        }
    }
    "?".to_string()
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
        return format!("Pushing code in {}", extract_project_from_path(title));
    }
    if title_lower.contains("git pull") || title_lower.contains("git fetch") {
        return format!("Pulling changes in {}", extract_project_from_path(title));
    }
    if title_lower.contains("npm install") || title_lower.contains("yarn add") || title_lower.contains("pnpm add") {
        return "Installing dependencies".to_string();
    }
    if title_lower.contains("npm run dev") || title_lower.contains("npm start") || title_lower.contains("cargo run") {
        return format!("{} dev server running", extract_project_from_path(title));
    }
    if title_lower.contains("ssh ") {
        return format!("{} remote session", app);
    }
    if title_lower.contains("docker") {
        return "Docker is running".to_string();
    }

    format!("{} terminal in {}", app, extract_project_from_path(title))
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
    if app_lower.contains("cursor") { "Cursor".to_string() }
    else if app_lower.contains("visual studio code") || app_lower == "code" { "VS Code".to_string() }
    else if app_lower.contains("windsurf") { "Windsurf".to_string() }
    else if app_lower.contains("trae") { "Trae".to_string() }
    else if app_lower.contains("zed") { "Zed".to_string() }
    else if app_lower.contains("xcode") { "Xcode".to_string() }
    else if app_lower.contains("sublime") { "Sublime".to_string() }
    else if app_lower.contains("intellij") { "IntelliJ".to_string() }
    else { capitalize(app) }
}

fn extract_browser_name(app: &str) -> String {
    let app_lower = app.to_lowercase();
    if app_lower.contains("chrome") { "Chrome".to_string() }
    else if app_lower == "arc" { "Arc".to_string() }
    else if app_lower.contains("safari") { "Safari".to_string() }
    else if app_lower.contains("firefox") { "Firefox".to_string() }
    else if app_lower.contains("brave") { "Brave".to_string() }
    else if app_lower.contains("edge") { "Edge".to_string() }
    else { capitalize(app) }
}

fn strip_browser_suffix(title: &str) -> String {
    let suffixes = [
        " - Google Chrome", " - Safari", " - Firefox", " - Microsoft Edge",
        " - Brave", " - Arc", " - Opera", " - Vivaldi", " \u{2014} Mozilla Firefox",
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

pub fn classify_source(app_name: &str) -> String {
    let lower = app_name.to_lowercase();
    if is_terminal(&lower) {
        "terminal".to_string()
    } else if is_browser(&lower) {
        "browser".to_string()
    } else if is_editor(&lower) {
        "editor".to_string()
    } else if is_chat(&lower) {
        "chat".to_string()
    } else {
        "other".to_string()
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}
