use tauri::{AppHandle, Manager, WebviewWindow, Emitter};
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
        // Check if menu is already visible and toggle it off
        if let Some(window) = &self.window {
            if window.is_visible().unwrap_or(false) {
                self.hide_menu();
                return;
            }
        }

        self.usage_data = data;

        // Get the existing window (created by Tauri at startup)
        let window = if let Some(w) = &self.window {
            w
        } else {
            // First time - get the window that Tauri already created
            match app.get_webview_window("menu") {
                Some(w) => {
                    self.window = Some(w.clone());
                    &self.window.as_ref().unwrap()
                }
                None => {
                    eprintln!("Menu window not found - make sure it's defined in tauri.conf.json");
                    return;
                }
            }
        };

        // Calculate window size
        let height = Self::calculate_height(&self.usage_data);
        let width = 280;

        // Set window size
        let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
            width,
            height,
        }));

        // TODO: Position window at top-right of screen below menubar
        // For now, window will appear at default position

        // Update and show the window
        let _ = window.emit("usage-data", &self.usage_data);
        let _ = window.show();
        let _ = window.set_focus();
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
