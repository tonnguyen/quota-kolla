use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, Emitter, WebviewWindowBuilder};
use crate::provider::ProviderUsage;
use std::sync::{Arc, Mutex};

/// Menu window state
pub struct MenuState {
    window: Option<WebviewWindow>,
    usage_data: Vec<ProviderUsage>,
}

impl MenuState {
    pub fn new() -> Self {
        Self {
            window: None,
            usage_data: Vec::new(),
        }
    }

    /// Show the dropdown menu at the correct position
    pub fn show_menu(&mut self, app: &AppHandle, data: Vec<ProviderUsage>) {
        self.usage_data = data;

        if let Some(window) = &self.window {
            // Update existing window and show
            let _ = window.emit("usage-data", &self.usage_data);
            let _ = window.show();
            let _ = window.set_focus();
            return;
        }

        // Calculate height based on provider count
        let height = Self::calculate_height(&self.usage_data) as f64;
        let width = 280.0;

        match WebviewWindowBuilder::new(
            app,
            "menu",
            WebviewUrl::App("menu.html".into())
        )
        .title("Usage Menu")
        .inner_size(width, height)
        .resizable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .build() {
            Ok(w) => {
                let _ = w.emit("usage-data", &self.usage_data);
                self.window = Some(w);
            }
            Err(e) => eprintln!("Failed to create menu window: {}", e),
        }
    }

    /// Hide the dropdown menu
    pub fn hide_menu(&mut self) {
        if let Some(window) = &self.window {
            let _ = window.hide();
        }
    }

    /// Check if menu is currently visible
    pub fn is_visible(&self) -> bool {
        self.window.as_ref()
            .map(|w| w.is_visible().unwrap_or(false))
            .unwrap_or(false)
    }

    /// Calculate window height based on content
    fn calculate_height(data: &[ProviderUsage]) -> u32 {
        let base = 16;
        let per_provider = 80;
        let actions = 44;
        let min_height = 200;
        base + (data.len() as u32 * per_provider).max(min_height) + actions
    }
}

impl Default for MenuState {
    fn default() -> Self {
        Self::new()
    }
}

/// Global menu state accessor
pub fn get_menu_state(app: &AppHandle) -> Arc<Mutex<MenuState>> {
    app.state::<Arc<Mutex<MenuState>>>()
        .inner()
        .clone()
}
