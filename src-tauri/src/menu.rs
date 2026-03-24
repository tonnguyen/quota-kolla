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
    /// tray_rect: (x, y, width, height) in physical pixels of the tray icon
    pub fn show_menu(&mut self, app: &AppHandle, data: Vec<ProviderUsage>, tray_rect: Option<(f64, f64, f64, f64)>) {
        // Check if menu is already visible and toggle it off
        if let Some(window) = &self.window {
            let visible = window.is_visible().unwrap_or(false);
            if visible {
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
                    eprintln!("Menu window 'menu' not found");
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

        // Position window near tray icon (bottom-right aligned)
        if let Some((tray_x, tray_y, tray_w, tray_h)) = tray_rect {
            if tray_x > 0.0 || tray_y > 0.0 {
                // Place below the menubar, right-aligned to tray icon
                let win_x = (tray_x + tray_w) as i32 - width as i32;
                let win_y = (tray_y + tray_h) as i32;
                let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                    x: win_x,
                    y: win_y,
                }));
            }
        }

        // Show window, emit shown timestamp, then emit data
        let _ = window.show();
        let _ = window.set_focus();

        // Emit 'shown' so JS can guard against immediate focus-lost hide
        let _ = window.emit("shown", ());
        let _ = window.emit("usage-data", &self.usage_data);
    }

    /// Hide the dropdown menu
    pub fn hide_menu(&mut self) {
        if let Some(window) = &self.window {
            let _ = window.hide();
        }
    }

    /// Get current usage data
    pub fn get_usage_data(&self) -> Vec<ProviderUsage> {
        self.usage_data.clone()
    }

    /// Update usage data and emit to window (for background updates)
    pub fn update_usage_data(&mut self, data: Vec<ProviderUsage>) {
        self.usage_data = data.clone();

        // Resize window if it's visible
        if let Some(window) = &self.window {
            if window.is_visible().unwrap_or(false) {
                let height = Self::calculate_height(&self.usage_data);
                let width = 280;
                let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize { width, height }));
            }

            let _ = window.emit("usage-data", &data);
        }
    }

    /// Update internal data without emitting (for background thread)
    pub fn set_usage_data(&mut self, data: Vec<ProviderUsage>) {
        self.usage_data = data;
    }

    /// Check if menu is currently visible
    pub fn is_visible(&self) -> bool {
        self.window.as_ref()
            .map(|w| w.is_visible().unwrap_or(false))
            .unwrap_or(false)
    }

    /// Calculate window height based on content
    fn calculate_height(data: &[ProviderUsage]) -> u32 {
        // Count visible rows in the provider area.
        let usage_lines: usize = data.iter()
            .map(|p| {
                if p.error.is_some() {
                    1
                } else if !p.usage_windows.is_empty() {
                    p.usage_windows.len()
                } else {
                    let mut legacy_count = 0;
                    if p.five_hour.is_some() { legacy_count += 1; }
                    if p.seven_day.is_some() { legacy_count += 1; }
                    if p.seven_day_opus.is_some() { legacy_count += 1; }
                    if p.seven_day_sonnet.is_some() { legacy_count += 1; }
                    legacy_count.max(1)
                }
            })
            .sum();

        let providers_padding = 16u32;
        let separator = 9u32;
        let menu_items = 68u32;
        let line_height = 42u32;

        providers_padding + (usage_lines as u32 * line_height) + separator + menu_items
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
