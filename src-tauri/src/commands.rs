use crate::ax_reader;
use crate::gemini;
use crate::heuristics;
use crate::state::{ActivityRecord, ClickableItem, OverlayContent, SharedState};
use crate::window_enum;
use crate::window_focus;
use sha2::{Digest, Sha256};
use std::time::Instant;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn get_overlay_content(state: tauri::State<'_, SharedState>) -> OverlayContent {
    let state = state.lock().unwrap();
    state.cached_content.clone()
}

#[tauri::command]
pub fn focus_window(app_name: String) {
    window_focus::focus_window(&app_name);
}

#[tauri::command]
pub fn dismiss_overlay(app_handle: AppHandle, state: tauri::State<'_, SharedState>) {
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.hide();
    }
    let mut state = state.lock().unwrap();
    state.overlay_visible = false;
}

#[tauri::command]
pub fn toggle_context(state: tauri::State<'_, SharedState>) -> bool {
    let mut state = state.lock().unwrap();
    state.context_enabled = !state.context_enabled;
    state.cached_content.context_enabled = state.context_enabled;
    // Reset hash to force a re-poll with new context level
    state.last_hash = String::new();
    state.context_enabled
}

pub fn toggle_overlay(app_handle: &AppHandle) {
    let state = app_handle.state::<SharedState>();
    let mut state_guard = state.lock().unwrap();

    if state_guard.overlay_visible {
        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.hide();
        }
        state_guard.overlay_visible = false;
    } else {
        if let Some(window) = app_handle.get_webview_window("main") {
            if let Ok(Some(monitor)) = window.current_monitor() {
                let monitor_size = monitor.size();
                let monitor_pos = monitor.position();
                let window_size = window.outer_size().unwrap_or(tauri::PhysicalSize {
                    width: 400,
                    height: 500,
                });
                let x =
                    monitor_pos.x + monitor_size.width as i32 - window_size.width as i32 - 16;
                let y = monitor_pos.y + 16;
                let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
            }
            let _ = window.show();

            // Force window to front — needed to appear over fullscreen apps
            #[cfg(target_os = "macos")]
            {
                let _ = window.with_webview(|webview| {
                    unsafe {
                        let ns_window: *mut objc2_app_kit::NSWindow =
                            webview.ns_window().cast();
                        if !ns_window.is_null() {
                            let ns_win = &*ns_window;
                            ns_win.orderFrontRegardless();
                        }
                    }
                });
            }
        }
        state_guard.overlay_visible = true;

        if !state_guard.is_polling {
            state_guard.is_polling = true;
            let handle = app_handle.clone();
            drop(state_guard);
            start_polling(handle);
        }
    }
}

fn start_polling(app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            {
                let state = app_handle.state::<SharedState>();
                let guard = state.lock().unwrap();
                if !guard.overlay_visible {
                    break;
                }
            }

            let context_enabled = {
                let state = app_handle.state::<SharedState>();
                let guard = state.lock().unwrap();
                guard.context_enabled
            };

            let mut windows = window_enum::enumerate_windows();
            for w in &mut windows {
                heuristics::categorize(w);
            }

            let deep = if context_enabled {
                ax_reader::read_deep_context(&windows)
            } else {
                ax_reader::DeepContext {
                    browsers: Vec::new(),
                    terminals: Vec::new(),
                    shell_sessions: Vec::new(),
                    foreground: None,
                    git_repos: Vec::new(),
                    notifications: Vec::new(),
                    recent_commands: Vec::new(),
                }
            };

            let mut hasher = Sha256::new();
            for w in &windows {
                hasher.update(format!("{}:{}", w.app_name, w.title));
            }
            for b in &deep.browsers {
                for tab in &b.tabs {
                    hasher.update(&tab.url);
                }
            }
            for t in &deep.terminals {
                let last_lines: String = t
                    .visible_text
                    .lines()
                    .rev()
                    .take(5)
                    .collect::<Vec<_>>()
                    .join("\n");
                hasher.update(&last_lines);
            }
            if let Some(fg) = &deep.foreground {
                hasher.update(&fg.app_name);
                let fg_lines: String = fg
                    .focused_text
                    .lines()
                    .rev()
                    .take(3)
                    .collect::<Vec<_>>()
                    .join("\n");
                hasher.update(&fg_lines);
                hasher.update(&fg.selected_text);
            }
            for g in &deep.git_repos {
                hasher.update(&g.branch);
                hasher.update(&g.status_short);
            }
            for s in &deep.shell_sessions {
                hasher.update(&s.cwd);
                hasher.update(&s.running_command);
            }
            for cmd in &deep.recent_commands {
                hasher.update(cmd);
            }
            hasher.update(if context_enabled { "ctx:on" } else { "ctx:off" });
            let hash = hex::encode(hasher.finalize());

            let (hash_changed, api_key, history_titles) = {
                let state = app_handle.state::<SharedState>();
                let mut guard = state.lock().unwrap();
                let changed = guard.last_hash != hash;

                let now = Instant::now();

                for w in &windows {
                    if let Some(existing) = guard
                        .activity_history
                        .iter_mut()
                        .find(|a| a.display_text == w.display_text)
                    {
                        existing.last_seen = now;
                    } else {
                        guard.activity_history.push_back(ActivityRecord {
                            display_text: w.display_text.clone(),
                            app_name: w.app_name.clone(),
                            last_seen: now,
                        });
                    }
                }

                let cutoff = std::time::Duration::from_secs(300);
                guard
                    .activity_history
                    .retain(|a| now.duration_since(a.last_seen) < cutoff);

                if changed {
                    for w in &windows {
                        let entry = format!("{}: {}", w.app_name, w.title);
                        if !guard.title_history.contains(&entry) {
                            if guard.title_history.len() >= 100 {
                                guard.title_history.pop_front();
                            }
                            guard.title_history.push_back(entry);
                        }
                    }
                }

                let titles: Vec<String> = guard.title_history.iter().cloned().collect();
                let key = guard.gemini_api_key.clone();
                (changed, key, titles)
            };

            if hash_changed {
                let state = app_handle.state::<SharedState>();
                let content = {
                    let guard = state.lock().unwrap();
                    heuristics::build_content(&windows, &deep, &guard.activity_history)
                };

                {
                    let mut guard = state.lock().unwrap();
                    guard.cached_content = content;
                    guard.cached_content.context_enabled = guard.context_enabled;
                    guard.last_hash = hash;
                    guard.windows = windows.clone();
                    guard.deep_context = Some(deep.clone());
                }

                if let Some(key) = api_key.filter(|_| context_enabled) {
                    let current: Vec<String> = windows
                        .iter()
                        .map(|w| format!("{}: {}", w.app_name, w.title))
                        .collect();
                    let mut tab_context: Vec<String> = Vec::new();
                    for b in &deep.browsers {
                        for tab in &b.tabs {
                            tab_context
                                .push(format!("[{}] {} - {}", b.app_name, tab.title, tab.url));
                        }
                    }
                    let mut terminal_context: Vec<String> = Vec::new();
                    for t in &deep.terminals {
                        let snippet: String = t
                            .visible_text
                            .lines()
                            .rev()
                            .take(30)
                            .collect::<Vec<_>>()
                            .into_iter()
                            .rev()
                            .collect::<Vec<_>>()
                            .join("\n");
                        terminal_context
                            .push(format!("[{} terminal]\n{}", t.app_name, snippet));
                    }

                    let foreground_context = if let Some(fg) = &deep.foreground {
                        let mut parts = vec![format!("[Active app: {}]", fg.app_name)];
                        if !fg.focused_text.is_empty() {
                            let snippet: String = fg
                                .focused_text
                                .lines()
                                .rev()
                                .take(25)
                                .collect::<Vec<_>>()
                                .into_iter()
                                .rev()
                                .collect::<Vec<_>>()
                                .join("\n");
                            parts.push(format!("Currently writing:\n{}", snippet));
                        }
                        if !fg.selected_text.is_empty() {
                            parts.push(format!(
                                "Selected text: {}",
                                if fg.selected_text.len() > 200 {
                                    format!("{}...", &fg.selected_text[..200])
                                } else {
                                    fg.selected_text.clone()
                                }
                            ));
                        }
                        parts.join("\n")
                    } else {
                        String::new()
                    };

                    let git_context: Vec<String> = deep
                        .git_repos
                        .iter()
                        .map(|g| {
                            let mut parts = vec![format!("[Git: {}]", g.repo_name)];
                            parts.push(format!("Branch: {}", g.branch));
                            if !g.status_short.is_empty() {
                                parts.push(format!("Changes:\n{}", g.status_short));
                            }
                            if !g.last_commit.is_empty() {
                                parts.push(format!("Last commit: {}", g.last_commit));
                            }
                            parts.join("\n")
                        })
                        .collect();

                    let notification_context: Vec<String> = deep
                        .notifications
                        .iter()
                        .map(|n| format!("[{}] {}", n.app_name, n.text))
                        .collect();

                    let shell_context: Vec<String> = deep
                        .shell_sessions
                        .iter()
                        .map(|s| {
                            let proj_info = if s.project_description.is_empty() {
                                s.project_name.clone()
                            } else {
                                format!("{} ({})", s.project_name, s.project_description)
                            };
                            if s.running_command.is_empty() {
                                format!(
                                    "[{} terminal] {} | dir: {} (idle)",
                                    s.terminal_app, proj_info, s.cwd
                                )
                            } else {
                                format!(
                                    "[{} terminal] {} | dir: {} — running: {}",
                                    s.terminal_app, proj_info, s.cwd, s.running_command
                                )
                            }
                        })
                        .collect();

                    let recent_cmds = deep.recent_commands.clone();

                    let handle = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Some(ai_items) = gemini::get_items(
                            &key,
                            &current,
                            &history_titles,
                            &tab_context,
                            &terminal_context,
                            &foreground_context,
                            &git_context,
                            &notification_context,
                            &shell_context,
                            &recent_cmds,
                        )
                        .await
                        {
                            let state = handle.state::<SharedState>();
                            let mut guard = state.lock().unwrap();
                            guard.cached_content.items = ai_items
                                .into_iter()
                                .map(|ai| {
                                    let source = heuristics::classify_source(&ai.app);
                                    ClickableItem {
                                        id: format!("ai:{}", ai.text),
                                        label: ai.text,
                                        app_name: ai.app,
                                        source_type: source,
                                        is_stale: false,
                                    }
                                })
                                .collect();
                        }
                    });
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }

        let state = app_handle.state::<SharedState>();
        let mut guard = state.lock().unwrap();
        guard.is_polling = false;
    });
}
