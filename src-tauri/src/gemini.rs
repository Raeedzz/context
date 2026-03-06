use serde_json::json;

pub async fn get_summary(
    api_key: &str,
    current_windows: &[String],
    history: &[String],
) -> Option<String> {
    let current_text = current_windows.join("\n");

    // Build history context (only titles not in current)
    let history_only: Vec<&String> = history
        .iter()
        .filter(|h| !current_windows.contains(h))
        .collect();
    let history_text = if history_only.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nRecently closed/switched away from:\n{}",
            history_only
                .iter()
                .rev()
                .take(20)
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    let body = json!({
        "contents": [{
            "parts": [{
                "text": format!(
                    "You are a concise productivity tracker. Based on the user's currently open windows and recent activity, write 1-2 short sentences about what they're working on and what they should focus on next. Be specific and actionable. Speak directly to the user (\"you\"). Don't list windows, synthesize them into tasks.\n\nCurrently open:\n{}{}\n\nRespond with just the summary, nothing else.",
                    current_text,
                    history_text
                )
            }]
        }],
        "generationConfig": {
            "maxOutputTokens": 100,
            "temperature": 0.3
        }
    });

    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash-lite:generateContent?key={}",
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

    json["candidates"]
        .get(0)?["content"]["parts"]
        .get(0)?["text"]
        .as_str()
        .map(|s| s.trim().to_string())
}
