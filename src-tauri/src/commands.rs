use crate::gemini;
use crate::heuristics;
use crate::state::{ActivityRecord, OverlayContent, SharedState};
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
            // Position top-right
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
            // Move window to current Space, then show once
            #[cfg(target_os = "macos")]
            {
                let _ = window.with_webview(|webview| {
                    use objc2_app_kit::NSWindowCollectionBehavior;
                    unsafe {
                        let ns_window: *mut objc2_app_kit::NSWindow =
                            webview.ns_window().cast();
                        if !ns_window.is_null() {
                            let ns_win = &*ns_window;
                            ns_win.setCollectionBehavior(
                                NSWindowCollectionBehavior::MoveToActiveSpace
                                    | NSWindowCollectionBehavior::IgnoresCycle,
                            );
                        }
                    }
                });
            }
            let _ = window.show();
        }
        state_guard.overlay_visible = true;

        let handle = app_handle.clone();
        drop(state_guard);
        start_polling(handle);
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

            let mut windows = window_enum::enumerate_windows();
            for w in &mut windows {
                heuristics::categorize(w);
            }

            // Hash current state
            let mut hasher = Sha256::new();
            for w in &windows {
                hasher.update(format!("{}:{}", w.app_name, w.title));
            }
            let hash = hex::encode(hasher.finalize());

            let state = app_handle.state::<SharedState>();
            let (hash_changed, api_key, history_titles) = {
                let mut guard = state.lock().unwrap();
                let changed = guard.last_hash != hash;

                let now = Instant::now();

                // Update activity history: add current windows, expire old entries
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

                // Remove entries older than 5 minutes
                let cutoff = std::time::Duration::from_secs(300);
                guard
                    .activity_history
                    .retain(|a| now.duration_since(a.last_seen) < cutoff);

                // Track title history for Gemini context (last 100)
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
                    heuristics::build_content(&windows, &guard.activity_history)
                };

                {
                    let mut guard = state.lock().unwrap();
                    let existing_summary = guard.cached_content.gemini_summary.clone();
                    guard.cached_content = content;
                    guard.cached_content.gemini_summary = existing_summary;
                    guard.last_hash = hash;
                    guard.windows = windows.clone();
                }

                // Gemini with full history for better context
                if let Some(key) = api_key {
                    let current: Vec<String> = windows
                        .iter()
                        .map(|w| format!("{}: {}", w.app_name, w.title))
                        .collect();
                    let handle = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Some(summary) =
                            gemini::get_summary(&key, &current, &history_titles).await
                        {
                            let state = handle.state::<SharedState>();
                            let mut guard = state.lock().unwrap();
                            guard.cached_content.gemini_summary = Some(summary);
                        }
                    });
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    });
}
