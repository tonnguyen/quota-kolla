// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            use tauri::tray::TrayIconBuilder;

            // 70% progress ring icon (pre-rendered from SVG):
            // - Circle radius: 9, center: (11, 11), stroke-width: 2
            // - Circumference: 2 * π * 9 ≈ 56.55
            // - 70% fill: 56.55 * 0.7 ≈ 39.58 (stroke-dasharray: "39.58 56.55")
            let png_bytes = include_bytes!("../icons/tray-icon-32.png");
            let img = image::load_from_memory(png_bytes)
                .expect("Failed to load tray icon PNG")
                .into_rgba8();
            let (width, height) = img.dimensions();
            let icon =
                tauri::image::Image::new_owned(img.into_raw(), width, height);

            TrayIconBuilder::new().icon(icon).build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
