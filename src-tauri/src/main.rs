// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod color;

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

// ── Theme detection ───────────────────────────────────────────────────

fn is_dark_mode() -> bool {
    Command::new("defaults")
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

// ── SVG generation ────────────────────────────────────────────────────

/// Build a 70×16 pt two-bar widget SVG.
/// Left: Claude 5h (yellow bar), Right: CCS 5h (red bar).
/// Adapts text/track colours for dark or light menu bar.
fn make_progress_svg(claude_pct: f64, ccs_pct: f64, dark: bool) -> String {
    let text_color = if dark { "#FFFFFF" } else { "#000000" };
    let track_color = if dark { "#48484A" } else { "#E5E5EA" };
    let divider_color = if dark { "#636366" } else { "#C7C7CC" };

    // Each section is 31px wide; bars start at x=1 and x=38
    let bar_w = 31.0_f64;
    let claude_fill = (bar_w * claude_pct / 100.0).clamp(0.0, bar_w);
    let ccs_fill = (bar_w * ccs_pct / 100.0).clamp(0.0, bar_w);

    format!(
        r##"<svg viewBox="0 0 70 16" width="70" height="16" xmlns="http://www.w3.org/2000/svg">
  <!-- Claude 5h (left) -->
  <text x="1" y="8.5" font-family="system-ui,-apple-system,Helvetica" font-size="6.5" font-weight="500" fill="{tc}">Claude 5h</text>
  <rect x="1" y="11" width="{bw}" height="3" rx="1.5" fill="{tkc}"/>
  <rect x="1" y="11" width="{cf:.1}" height="3" rx="1.5" fill="#FFD60A"/>
  <!-- Divider -->
  <line x1="36.5" y1="2" x2="36.5" y2="14" stroke="{dc}" stroke-width="0.5"/>
  <!-- CCS 5h (right) -->
  <text x="38" y="8.5" font-family="system-ui,-apple-system,Helvetica" font-size="6.5" font-weight="500" fill="{tc}">CCS 5h</text>
  <rect x="38" y="11" width="{bw}" height="3" rx="1.5" fill="{tkc}"/>
  <rect x="38" y="11" width="{gf:.1}" height="3" rx="1.5" fill="#FF453A"/>
</svg>"##,
        tc = text_color,
        tkc = track_color,
        dc = divider_color,
        bw = bar_w,
        cf = claude_fill,
        gf = ccs_fill,
    )
}

/// Render SVG to raw RGBA pixel data. Renders at 2× for Retina clarity.
fn render_svg_to_rgba(svg: &str, pt_width: u32, pt_height: u32) -> Option<Vec<u8>> {
    use resvg::{tiny_skia, usvg};

    let scale = 2_u32; // 2× Retina rendering
    let px_w = pt_width * scale;
    let px_h = pt_height * scale;

    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &opt).ok()?;

    let mut pixmap = tiny_skia::Pixmap::new(px_w, px_h)?;
    let sx = px_w as f32 / tree.size().width();
    let sy = px_h as f32 / tree.size().height();
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(sx, sy),
        &mut pixmap.as_mut(),
    );
    Some(pixmap.data().to_vec())
}

// ── Claude usage (Anthropic OAuth API) ───────────────────────────────

fn get_claude_token() -> Option<String> {
    let out = Command::new("security")
        .args(["find-generic-password", "-s", "Claude Code-credentials", "-w"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let raw = String::from_utf8(out.stdout).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(raw.trim()).ok()?;
    parsed["claudeAiOauth"]["accessToken"]
        .as_str()
        .map(|s| s.to_string())
}

fn fetch_claude_utilization() -> Option<f64> {
    let token = get_claude_token()?;
    let resp = ureq::get("https://api.anthropic.com/api/oauth/usage")
        .set("Authorization", &format!("Bearer {token}"))
        .set("anthropic-beta", "oauth-2025-04-20")
        .call()
        .ok()?;
    let data: serde_json::Value = resp.into_json().ok()?;
    data["five_hour"]["utilization"].as_f64()
}

// ── CCS / GLM usage (Z.AI API) ────────────────────────────────────────

fn get_glm_api_key() -> Option<String> {
    let path = PathBuf::from(std::env::var("HOME").ok()?)
        .join(".ccs")
        .join("glm.settings.json");
    let content = fs::read_to_string(path).ok()?;
    let s: serde_json::Value = serde_json::from_str(&content).ok()?;
    s["env"]["ANTHROPIC_AUTH_TOKEN"]
        .as_str()
        .or_else(|| s["env"]["GLM_API_KEY"].as_str())
        .or_else(|| s["ANTHROPIC_AUTH_TOKEN"].as_str())
        .or_else(|| s["GLM_API_KEY"].as_str())
        .map(|v| v.to_string())
}

fn fetch_ccs_utilization() -> Option<f64> {
    let key = get_glm_api_key()?;
    let resp = ureq::get("https://api.z.ai/api/monitor/usage/quota/limit")
        .set("Authorization", &format!("Bearer {key}"))
        .call()
        .ok()?;
    let data: serde_json::Value = resp.into_json().ok()?;
    if data["success"].as_bool() != Some(true) {
        return None;
    }
    data["data"]["limits"]
        .as_array()?
        .iter()
        .find(|l| l["type"].as_str() == Some("TOKENS_LIMIT"))
        .and_then(|l| l["percentage"].as_f64())
}

// ── Tray icon update ──────────────────────────────────────────────────

fn update_tray_icon(app: &AppHandle, claude_pct: f64, ccs_pct: f64, dark: bool) {
    let svg = make_progress_svg(claude_pct, ccs_pct, dark);
    let Some(rgba) = render_svg_to_rgba(&svg, 70, 16) else {
        eprintln!("Failed to render progress SVG");
        return;
    };
    // 2× render: 140×32 pixels for 70×16 pt display
    let icon = tauri::image::Image::new_owned(rgba, 140, 32);
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_icon(Some(icon));
        // Disable template mode so our colors are preserved
        let _ = tray.set_icon_as_template(false);
        println!("Tray updated: Claude={claude_pct:.1}% CCS={ccs_pct:.1}% dark={dark}");
    }
}

// ── Entry point ───────────────────────────────────────────────────────

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            use tauri::tray::TrayIconBuilder;

            let dark = is_dark_mode();
            let initial_svg = make_progress_svg(0.0, 0.0, dark);
            let initial_rgba =
                render_svg_to_rgba(&initial_svg, 70, 16).expect("Failed to render initial SVG");
            let initial_icon = tauri::image::Image::new_owned(initial_rgba, 140, 32);

            TrayIconBuilder::with_id("main")
                .icon(initial_icon)
                .icon_as_template(false)
                .build(app)?;

            // Cache last values to re-render on theme change without re-fetching
            let cached: Arc<Mutex<(f64, f64, bool)>> = Arc::new(Mutex::new((0.0, 0.0, dark)));

            // Background thread: fetch data every 5 min, check theme every 10 s
            let app_handle = app.handle().clone();
            let cache_clone = Arc::clone(&cached);
            std::thread::spawn(move || {
                let mut ticks = 0u32;
                loop {
                    // Fetch data every 30 ticks (= 5 min at 10 s intervals)
                    let (new_claude, new_ccs) = if ticks % 30 == 0 {
                        let c = fetch_claude_utilization().unwrap_or_else(|| {
                            eprintln!("Could not fetch Claude usage");
                            cache_clone.lock().unwrap().0
                        });
                        let g = fetch_ccs_utilization().unwrap_or_else(|| {
                            eprintln!("Could not fetch CCS usage");
                            cache_clone.lock().unwrap().1
                        });
                        (c, g)
                    } else {
                        let lock = cache_clone.lock().unwrap();
                        (lock.0, lock.1)
                    };

                    let new_dark = is_dark_mode();
                    let changed = {
                        let mut lock = cache_clone.lock().unwrap();
                        let changed = lock.0 != new_claude || lock.1 != new_ccs || lock.2 != new_dark;
                        *lock = (new_claude, new_ccs, new_dark);
                        changed
                    };

                    if changed || ticks == 0 {
                        update_tray_icon(&app_handle, new_claude, new_ccs, new_dark);
                    }

                    ticks += 1;
                    std::thread::sleep(std::time::Duration::from_secs(10));
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
