use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize)]
pub struct AiItem {
    pub text: String,
    pub app: String,
}

pub async fn get_items(
    api_key: &str,
    current_windows: &[String],
    history: &[String],
    browser_tabs: &[String],
    terminal_output: &[String],
    foreground_text: &str,
    git_status: &[String],
    notifications: &[String],
    shell_sessions: &[String],
    recent_commands: &[String],
) -> Option<Vec<AiItem>> {
    let current_text = current_windows.join("\n");

    let history_only: Vec<&String> = history
        .iter()
        .filter(|h| !current_windows.contains(h))
        .collect();
    let history_text = if history_only.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nRecently closed:\n{}",
            history_only
                .iter()
                .rev()
                .take(15)
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    let tabs_text = if browser_tabs.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nAll browser tabs with URLs:\n{}",
            browser_tabs.join("\n")
        )
    };

    let terminal_text = if terminal_output.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nTerminal output:\n{}",
            terminal_output.join("\n---\n")
        )
    };

    let foreground_section = if foreground_text.is_empty() {
        String::new()
    } else {
        format!("\n\nForeground app content (what user is actively working on):\n{}", foreground_text)
    };

    let git_section = if git_status.is_empty() {
        String::new()
    } else {
        format!("\n\nGit repositories:\n{}", git_status.join("\n---\n"))
    };

    let notification_section = if notifications.is_empty() {
        String::new()
    } else {
        format!("\n\nRecent notifications:\n{}", notifications.join("\n"))
    };

    let shell_section = if shell_sessions.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nTerminal sessions (CWD + running processes):\n{}",
            shell_sessions.join("\n")
        )
    };

    let commands_section = if recent_commands.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nLast 3 terminal commands (most recent activity):\n{}",
            recent_commands.join("\n")
        )
    };

    let body = json!({
        "contents": [{
            "parts": [{
                "text": format!(
                    "You are watching someone's screen. Narrate what's happening at each window/terminal/tab.\n\nFor each activity, write ONE short phrase describing what's happening — be specific about WHAT is being worked on, not just which tool is running.\n\nExamples of good phrases:\n- \"claude working on context overlay — adding color-coded terminal items\"\n- \"dev server running sckry api on localhost:3000\"\n- \"editing gemini.rs — writing the prompt builder\"\n- \"3 github tabs open for tauri docs\"\n- \"scraper throwing 'connection refused' in extension_capture\"\n- \"spotify playing in background\"\n- \"on feature/overlay branch — 5 uncommitted files\"\n\nReturn a JSON array. Each element: {{\"text\": \"your phrase\", \"app\": \"ExactAppName\"}}\n\nIMPORTANT for terminal sessions:\n- When \"claude\" is running in a directory, say what claude is working on based on the project name, git changes, and recent commits — e.g. \"claude working on context — adding terminal reading\" NOT just \"running claude\"\n- When a dev server is running (bun run dev, npm exec), say what project it's serving — e.g. \"sckry dev server running\" NOT just \"running bun\"\n- Combine git status with terminal info: if there are uncommitted changes in a project where claude is running, mention what changed\n- Each terminal session with a different project deserves its own line\n\nRules:\n- Use the EXACT app name from the window list or terminal app name (case-sensitive)\n- Read terminal output carefully — report errors, build status, test results\n- Read what they're typing — mention actual code/text\n- Group browser tabs by purpose (don't list every tab)\n- Be specific not generic\n- Lowercase, natural, short\n- Max 12 items, order by relevance\n\nOpen windows:\n{}{}{}{}{}{}{}{}{}\n\nRespond with ONLY the JSON array, no markdown, no explanation.",
                    current_text,
                    tabs_text,
                    terminal_text,
                    shell_section,
                    commands_section,
                    foreground_section,
                    git_section,
                    notification_section,
                    history_text
                )
            }]
        }],
        "generationConfig": {
            "maxOutputTokens": 500,
            "temperature": 0.3
        }
    });

    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-flash-lite-latest:generateContent?key={}",
        api_key
    );

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .ok()?;

    let json: serde_json::Value = response.json().await.ok()?;

    let raw = json["candidates"]
        .get(0)?["content"]["parts"]
        .get(0)?["text"]
        .as_str()?;

    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    serde_json::from_str::<Vec<AiItem>>(cleaned).ok()
}
