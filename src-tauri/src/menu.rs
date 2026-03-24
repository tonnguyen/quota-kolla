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
        eprintln!("[DEBUG] MenuState::show_menu called with {} providers", data.len());

        // Check if menu is already visible and toggle it off
        if let Some(window) = &self.window {
            let visible = window.is_visible().unwrap_or(false);
            eprintln!("[DEBUG] MenuState::show_menu: cached window exists, is_visible={}", visible);
            if visible {
                eprintln!("[DEBUG] MenuState::show_menu: hiding visible menu");
                self.hide_menu();
                return;
            }
        } else {
            eprintln!("[DEBUG] MenuState::show_menu: no cached window yet");
        }

        self.usage_data = data;

        // Get the existing window (created by Tauri at startup)
        let window = if let Some(w) = &self.window {
            eprintln!("[DEBUG] MenuState::show_menu: using cached window");
            w
        } else {
            // First time - get the window that Tauri already created
            eprintln!("[DEBUG] MenuState::show_menu: looking up 'menu' window from app");
            match app.get_webview_window("menu") {
                Some(w) => {
                    eprintln!("[DEBUG] MenuState::show_menu: found 'menu' window");
                    self.window = Some(w.clone());
                    &self.window.as_ref().unwrap()
                }
                None => {
                    eprintln!("[DEBUG][ERROR] Menu window 'menu' not found - make sure it's defined in tauri.conf.json");
                    return;
                }
            }
        };

        // Log current window position BEFORE show
        if let Ok(pos) = window.outer_position() {
            eprintln!("[DEBUG] MenuState::show_menu: window position BEFORE show: {:?}", pos);
        }
        if let Ok(size) = window.outer_size() {
            eprintln!("[DEBUG] MenuState::show_menu: window outer_size BEFORE show: {:?}", size);
        }

        // Calculate window size
        let height = Self::calculate_height(&self.usage_data);
        let width = 280;
        eprintln!("[DEBUG] MenuState::show_menu: setting size {}x{}", width, height);

        // Set window size
        let size_result = window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
            width,
            height,
        }));
        eprintln!("[DEBUG] MenuState::show_menu: set_size result: {:?}", size_result);

        // Position window near tray icon (bottom-right aligned)
        if let Some((tray_x, tray_y, tray_w, tray_h)) = tray_rect {
            if tray_x > 0.0 || tray_y > 0.0 {
                // Place below the menubar, right-aligned to tray icon
                let win_x = (tray_x + tray_w) as i32 - width as i32;
                let win_y = (tray_y + tray_h) as i32;
                eprintln!("[DEBUG] MenuState::show_menu: positioning at physical ({}, {})", win_x, win_y);
                let pos_result = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                    x: win_x,
                    y: win_y,
                }));
                eprintln!("[DEBUG] MenuState::show_menu: set_position result: {:?}", pos_result);
            }
        }

        // Show window, emit shown timestamp, then emit data
        let show_result = window.show();
        eprintln!("[DEBUG] MenuState::show_menu: show() result: {:?}", show_result);
        let focus_result = window.set_focus();
        eprintln!("[DEBUG] MenuState::show_menu: set_focus() result: {:?}", focus_result);

        // Log position AFTER show
        if let Ok(pos) = window.outer_position() {
            eprintln!("[DEBUG] MenuState::show_menu: window position AFTER show: {:?}", pos);
        }

        // Emit 'shown' so JS can guard against immediate focus-lost hide
        let _ = window.emit("shown", ());
        let emit_result = window.emit("usage-data", &self.usage_data);
        eprintln!("[DEBUG] MenuState::show_menu: emit('usage-data') result: {:?}", emit_result);
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
