// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::SystemTray;
use tauri::AppHandle;
use tauri::tray::SystemTrayIcon;

fn main() {
    // Create the 70% progress ring SVG as per design spec
    // - Size: 22x22 viewBox
    // - Circle radius: 9, center at (11, 11)
    // - Stroke width: 2
    // - Circumference: 2 * π * 9 ≈ 56.55
    // - 70% fill: 56.55 * 0.7 ≈ 39.58

    const SVG_ICON: &str = r#"<svg viewBox="0 0 22 22" width="22" height="22" xmlns="http://www.w3.org/2000/svg">
    <defs>
      <style>
        .track { stroke: #D1D1D6; }
        .progress { stroke: #007AFF; }
      </style>
    </defs>
    <!-- Background track (gray circle) -->
    <circle cx="11" cy="11" r="9" fill="none" stroke-width="2"
            stroke-dasharray="56.55" stroke-dashoffset="0"
            class="track" />
    <!-- Progress fill (70% blue, rotated -90deg to start from top) -->
    <circle cx="11" cy="11" r="9" fill="none" stroke-width="2"
            stroke-dasharray="39.58 56.55" stroke-dashoffset="0"
            transform="rotate(-90 11 11)"
            class="progress" />
  </svg>"#;

    // Create system tray (icon will be set after app starts)
    let tray = SystemTray::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .system_tray(tray)
        .setup(|app| {
            // Set the tray icon after app initialization
            set_tray_icon(app, SVG_ICON);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Sets the tray icon from an SVG string
/// Uses macOS template mode for automatic dark/light appearance
fn set_tray_icon(app: &tauri::AppHandle, svg: &str) {
    use tauri::tray::SystemTrayIcon;

    // Convert SVG string to bytes
    let svg_bytes = svg.as_bytes().to_vec();

    // Create icon from raw SVG bytes
    // On macOS, this will use template mode automatically
    let icon = SystemTrayIcon::Raw(svg_bytes);

    // Get the tray handle and set the icon
    let tray_handle = app.tray_handle();
    if let Err(e) = tray_handle.set_icon(icon) {
        eprintln!("Failed to set tray icon: {}", e);

        // Fallback: try with a simpler SVG
        let fallback_svg = r#"<svg viewBox="0 0 22 22" xmlns="http://www.w3.org/2000/svg">
    <circle cx="11" cy="11" r="9" fill="#007AFF"/>
  </svg>"#;
        let fallback_icon = SystemTrayIcon::Raw(fallback_svg.as_bytes().to_vec());
        let _ = tray_handle.set_icon(fallback_icon);
    } else {
        println!("Menu bar icon set: 22x22 progress ring at 70%");
    }
}
