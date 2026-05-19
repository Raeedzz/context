use crate::ax_reader::DeepContext;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: String,
    pub app_name: String,
    pub title: String,
    pub category: String,
    pub display_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickableItem {
    pub id: String,
    pub label: String,
    pub app_name: String,
    pub source_type: String,
    pub is_stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayContent {
    pub markdown: String,
    pub items: Vec<ClickableItem>,
    pub context_enabled: bool,
}

impl Default for OverlayContent {
    fn default() -> Self {
        Self {
            markdown: String::from("Scanning windows..."),
            items: Vec::new(),
            context_enabled: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActivityRecord {
    pub display_text: String,
    pub app_name: String,
    pub last_seen: Instant,
}

pub struct AppState {
    pub windows: Vec<WindowInfo>,
    pub last_hash: String,
    pub cached_content: OverlayContent,
    pub gemini_api_key: Option<String>,
    pub overlay_visible: bool,
    pub is_polling: bool,
    pub activity_history: VecDeque<ActivityRecord>,
    pub title_history: VecDeque<String>,
    pub deep_context: Option<DeepContext>,
    pub context_enabled: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            windows: Vec::new(),
            last_hash: String::new(),
            cached_content: OverlayContent::default(),
            gemini_api_key: std::env::var("CONTEXT_GEMINI_API_KEY").ok(),
            overlay_visible: false,
            is_polling: false,
            activity_history: VecDeque::new(),
            title_history: VecDeque::with_capacity(100),
            deep_context: None,
            context_enabled: false,
        }
    }
}

pub type SharedState = Mutex<AppState>;
