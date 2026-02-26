use std::sync::Mutex;
use photo_selector_core::app_state::AppState;

/// AppState wrapped in a Mutex so Tauri command handlers
/// can safely acquire it across async boundaries.
///
/// Default view_count of 1 — the GUI will call set_view_count
/// once it knows the grid dimensions.
pub struct TauriAppState(pub Mutex<AppState>);

impl TauriAppState {
    pub fn new() -> Self {
        Self(Mutex::new(AppState::new(1)))
    }
}