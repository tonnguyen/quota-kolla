// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Listener, Manager};

mod color;
mod config;
mod provider;
mod render;
mod menu;

use config::Config;
use provider::all_providers;
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

        let provider_info: Vec<String> = providers
            .iter()
            .map(|(name, usage, _)| format!("{}={:.1}%", name, usage))
            .collect();
        println!("Tray updated: {} dark={}", provider_info.join(", "), dark);
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

            // Initial icon with zero usage
            let providers: Vec<(String, f64, config::DisplayMode)> = cfg
                .visible_providers()
                .iter()
                .map(|id| (id.clone(), 0.0, cfg.providers.get(id).unwrap().get_mode()))
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
                        ..
                    } = event {
                        let _ = app_handle_for_tray.emit("tray-click", ());
                    }
                })
                .build(app)?;

            // Listen for tray clicks to show menu
            let app_handle_for_events = app.handle().clone();
            app.listen("tray-click", move |_| {
                show_menu(app_handle_for_events.clone());
            });

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
            std::thread::spawn(move || {
                let providers = all_providers();
                let mut ticks = 0u32;
                loop {
                    // Fetch data every 30 ticks (= 5 min at 10 s intervals)
                    let usage_changed = if ticks % 30 == 0 {
                        let mut changed = false;
                        for provider in &providers {
                            if let Some(usage) = provider.fetch_usage() {
                                let old = cached_usage.get(provider.id()).copied().unwrap_or(0.0);
                                if (old - usage).abs() > 0.1 {
                                    cached_usage.insert(provider.id().to_string(), usage);
                                    changed = true;
                                }
                            } else {
                                eprintln!("Could not fetch {} usage", provider.id());
                            }
                        }
                        changed
                    } else {
                        false
                    };

                    let new_dark = is_dark_mode();

                    if usage_changed || ticks == 0 {
                        let cfg = config_clone.lock().unwrap();
                        let visible_providers: Vec<(String, f64, config::DisplayMode)> = cfg
                            .visible_providers()
                            .iter()
                            .filter_map(|id| {
                                let usage = *cached_usage.get(id)?;
                                let mode = cfg.providers.get(id)?.get_mode();
                                Some((id.clone(), usage, mode))
                            })
                            .collect();

                        update_tray_icon(&app_handle, &visible_providers, &cfg, new_dark);
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn show_menu(app: AppHandle) {
    let providers = crate::provider::fetch_all_usage();
    let menu_state = menu::get_menu_state(&app);
    let mut state = menu_state.lock().unwrap();
    state.show_menu(&app, providers);
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
