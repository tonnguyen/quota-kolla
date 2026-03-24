// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

mod color;
mod config;
mod provider;
mod render;
mod menu;

use config::Config;
use provider::{all_providers, fetch_all_usage};
use render::{build_full_svg, render_svg_to_rgba};
use menu::MenuState;

// ── Theme detection ───────────────────────────────────────────────────

fn is_dark_mode() -> bool {
    std::process::Command::new("defaults")
        .args(["read", "-g", "AppleInterfaceStyle"])
        .output()
        .map(|o| {
            o.status.success()
                && String::from_utf8_lossy(&o.stdout)
                    .trim()
                    .eq_ignore_ascii_case("dark")
        })
        .unwrap_or(false)
}

// ── Tray icon update ──────────────────────────────────────────────────

fn update_tray_icon(
    app: &AppHandle,
    providers: &[(String, f64, config::DisplayMode)],
    config: &Config,
    dark: bool,
) {
    let width = config.total_width();
    let height = config.max_height();

    let svg = build_full_svg(providers, dark);
    let Some(rgba) = render_svg_to_rgba(&svg, width, height) else {
        eprintln!("Failed to render progress SVG");
        return;
    };

    let icon = tauri::image::Image::new_owned(rgba, width * 2, height * 2);
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_icon(Some(icon));
        let _ = tray.set_icon_as_template(false);
    }
}

// ── Entry point ───────────────────────────────────────────────────────

fn main() {
    // Load config on startup
    let config = Arc::new(Mutex::new(Config::load()));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
            // Initialize menu state
            let menu_state = Arc::new(Mutex::new(MenuState::new()));
            app.manage(menu_state.clone());
            use tauri::tray::TrayIconBuilder;

            let cfg = config.lock().unwrap();
            let dark = is_dark_mode();
            let width = cfg.total_width();
            let height = cfg.max_height();

            // Provider display names mapping
            let display_names: std::collections::HashMap<&str, &str> = [
                ("claude", "Claude"),
                ("glm", "zAI"),
                ("codex", "Codex"),
            ].into_iter().collect();

            let get_display_name = |id: &str| -> String {
                display_names.get(id).copied().unwrap_or(id).to_string()
            };

            // Initial icon with zero usage
            let providers: Vec<(String, f64, config::DisplayMode)> = cfg
                .visible_providers()
                .iter()
                .map(|id| (get_display_name(id), 0.0, cfg.providers.get(id).unwrap().get_mode()))
                .collect();

            let initial_svg = build_full_svg(&providers, dark);
            let initial_rgba = render_svg_to_rgba(&initial_svg, width, height)
                .expect("Failed to render initial SVG");
            let initial_icon = tauri::image::Image::new_owned(initial_rgba, width * 2, height * 2);

            let app_handle_for_tray = app.handle().clone();
            TrayIconBuilder::with_id("main")
                .icon(initial_icon)
                .icon_as_template(false)
                .on_tray_icon_event(move |_tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        rect,
                        ..
                    } = event {
                        // Extract tray icon rect in physical pixels
                        let (tray_x, tray_y) = match rect.position {
                            tauri::Position::Physical(p) => (p.x as f64, p.y as f64),
                            tauri::Position::Logical(p) => (p.x, p.y),
                        };
                        let (tray_w, tray_h) = match rect.size {
                            tauri::Size::Physical(s) => (s.width as f64, s.height as f64),
                            tauri::Size::Logical(s) => (s.width, s.height),
                        };
                        let app = app_handle_for_tray.clone();
                        std::thread::spawn(move || {
                            show_menu_at(app, tray_x, tray_y, tray_w, tray_h);
                        });
                    }
                })
                .build(app)?;

            // (tray-click now handled directly in on_tray_icon_event via show_menu_at)

            // Cache: (provider_usage_map, dark_mode)
            let all_provider_ids: Vec<String> = all_providers()
                .iter()
                .map(|p| p.id().to_string())
                .collect();

            let mut cached_usage: std::collections::HashMap<String, f64> = all_provider_ids
                .iter()
                .map(|id| (id.clone(), 0.0))
                .collect();

            // Background thread
            let app_handle = app.handle().clone();
            let config_clone = Arc::clone(&config);
            let menu_state_for_bg = Arc::clone(&menu_state);
            std::thread::spawn(move || {
                // Initial fetch for menu state
                let mut initial_providers = fetch_all_usage();
                initial_providers = initial_providers.into_iter()
                    .filter_map(|mut p| {
                        if p.error.as_ref().map_or(false, |e| e.contains("Not implemented")) {
                            return None;
                        }
                        if let Some(error) = &p.error {
                            if error.contains("Network error") || error.contains("status code") {
                                p.error = Some("Network error".to_string());
                            }
                        }
                        Some(p)
                    })
                    .collect();
                {
                    let mut state = menu_state_for_bg.lock().unwrap();
                    state.set_usage_data(initial_providers.clone());
                }

                // Continue with periodic updates
                let providers = all_providers();
                let mut ticks = 0u32;
                loop {
                    // Fetch data every 30 ticks (= 5 min at 10 s intervals)
                    let usage_changed = if ticks % 30 == 0 {
                        let mut changed = false;
                        for provider in &providers {
                            match provider.fetch_usage_data() {
                                Ok(data) => {
                                    if let Some(window) = data.five_hour {
                                        let old = cached_usage.get(provider.id()).copied().unwrap_or(0.0);
                                        if (old - window.utilization).abs() > 0.1 {
                                            cached_usage.insert(provider.id().to_string(), window.utilization);
                                            changed = true;
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Could not fetch {} usage: {}", provider.id(), e);
                                }
                            }
                        }
                        changed
                    } else {
                        false
                    };

                    let new_dark = is_dark_mode();

                    if usage_changed || ticks == 0 {
                        let cfg = config_clone.lock().unwrap();

                        // Provider display names mapping
                        let display_names: std::collections::HashMap<&str, &str> = [
                            ("claude", "Claude"),
                            ("glm", "zAI"),
                            ("codex", "Codex"),
                        ].into_iter().collect();

                        let visible_providers: Vec<(String, f64, config::DisplayMode)> = cfg
                            .visible_providers()
                            .iter()
                            .filter_map(|id| {
                                let usage = *cached_usage.get(id)?;
                                let mode = cfg.providers.get(id)?.get_mode();
                                let display_name = display_names.get(id.as_str()).copied().unwrap_or(id);
                                Some((display_name.to_string(), usage, mode))
                            })
                            .collect();

                        update_tray_icon(&app_handle, &visible_providers, &cfg, new_dark);

                        // Also update menu state with fresh data
                        let mut providers = fetch_all_usage();
                        providers = providers.into_iter()
                            .filter_map(|mut p| {
                                if p.error.as_ref().map_or(false, |e| e.contains("Not implemented")) {
                                    return None;
                                }
                                if let Some(error) = &p.error {
                                    if error.contains("Network error") || error.contains("status code") {
                                        p.error = Some("Network error".to_string());
                                    }
                                }
                                Some(p)
                            })
                            .collect();

                        let mut state = menu_state_for_bg.lock().unwrap();
                        state.set_usage_data(providers.clone());
                    }

                    ticks += 1;
                    std::thread::sleep(std::time::Duration::from_secs(10));
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            show_menu,
            hide_menu,
            quit_app,
            get_preferences,
            save_preferences,
            show_preferences,
            get_menu_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn show_menu_at(app: AppHandle, tray_x: f64, tray_y: f64, tray_w: f64, tray_h: f64) {
    // Get cached data immediately for instant display
    let menu_state = menu::get_menu_state(&app);
    let cached_data = {
        let state = menu_state.lock().unwrap();
        state.get_usage_data()
    };

    // Show menu immediately with cached data
    {
        let menu_state = menu::get_menu_state(&app);
        let mut state = menu_state.lock().unwrap();
        state.show_menu(&app, cached_data, Some((tray_x, tray_y, tray_w, tray_h)));
    }

    // Fetch fresh data in background and update
    let app_clone = app.clone();
    std::thread::spawn(move || {
        let mut providers = crate::provider::fetch_all_usage();

        // Filter out "Not implemented yet" errors and simplify network errors
        providers = providers.into_iter()
            .filter_map(|mut p| {
                if p.error.as_ref().map_or(false, |e| e.contains("Not implemented")) {
                    return None;
                }
                if let Some(error) = &p.error {
                    if error.contains("Network error") || error.contains("status code") {
                        p.error = Some("Network error".to_string());
                    }
                }
                Some(p)
            })
            .collect();

        // Update state and emit to window
        let menu_state = menu::get_menu_state(&app_clone);
        let mut state = menu_state.lock().unwrap();
        state.update_usage_data(providers);
    });
}

#[tauri::command]
fn show_menu(app: AppHandle) {
    show_menu_at(app, 0.0, 0.0, 0.0, 0.0);
}

#[tauri::command]
fn hide_menu(app: AppHandle) {
    let menu_state = menu::get_menu_state(&app);
    let mut state = menu_state.lock().unwrap();
    state.hide_menu();
}

#[tauri::command]
fn quit_app() {
    std::process::exit(0);
}

#[tauri::command]
async fn get_preferences() -> Result<config::Config, String> {
    Ok(config::Config::load())
}

#[tauri::command]
async fn save_preferences(config: config::Config) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_menu_data(app: AppHandle) -> Vec<provider::ProviderUsage> {
    let menu_state = menu::get_menu_state(&app);
    let state = menu_state.lock().unwrap();
    state.get_usage_data()
}

#[tauri::command]
async fn show_preferences(app: AppHandle) {
    if let Some(window) = app.get_webview_window("preferences") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}
