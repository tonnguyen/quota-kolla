use crate::color::get_usage_color;
use crate::config::DisplayMode;

/// Calculate circle stroke-dasharray for percentage
/// Returns (filled_length, circumference)
pub fn calculate_circle_dash_array(percentage: f64, radius: f64) -> (f64, f64) {
    let circumference = 2.0 * std::f64::consts::PI * radius;
    let filled = circumference * percentage.clamp(0.0, 100.0) / 100.0;
    (filled, circumference)
}

/// Render SVG for a single provider
pub fn render_provider_svg(
    provider_name: &str,
    usage: f64,
    mode: DisplayMode,
    dark: bool,
) -> String {
    let text_color = if dark { "#FFFFFF" } else { "#000000" };
    let track_color = if dark { "#48484A" } else { "#E5E5EA" };
    let usage_color = get_usage_color(usage);
    let pct_text = format!("{:.0}%", usage.round());

    match mode {
        DisplayMode::Bar => render_bar_svg(provider_name, usage, &pct_text, text_color, track_color, usage_color),
        DisplayMode::Text => render_text_svg(provider_name, &pct_text, text_color, usage_color),
        DisplayMode::Circle => render_circle_svg(usage, usage_color, track_color),
    }
}

fn render_bar_svg(name: &str, usage: f64, pct_text: &str, text_color: &str, track_color: &str, fill_color: &str) -> String {
    let bar_w = 31.0_f64;
    let fill_w = (bar_w * usage / 100.0).clamp(0.0, bar_w);
    let label = format!("{}: {}", name, pct_text);

    format!(
        r##"<svg viewBox="0 0 32 16" width="32" height="16" xmlns="http://www.w3.org/2000/svg">
  <text x="0" y="6.5" font-family="system-ui,-apple-system,Helvetica" font-size="6.5" font-weight="500" fill="{tc}">{label}</text>
  <rect x="0" y="11" width="{bw}" height="3" rx="1.5" fill="{tkc}"/>
  <rect x="0" y="11" width="{fw:.1}" height="3" rx="1.5" fill="{fc}"/>
</svg>"##,
        tc = text_color,
        tkc = track_color,
        fc = fill_color,
        bw = bar_w,
        fw = fill_w,
        label = label,
    )
}

fn render_text_svg(name: &str, pct_text: &str, _text_color: &str, fill_color: &str) -> String {
    let label = format!("{}: {}", name, pct_text);

    format!(
        r##"<svg viewBox="0 0 24 10" width="24" height="10" xmlns="http://www.w3.org/2000/svg">
  <text x="0" y="7" font-family="system-ui,-apple-system,Helvetica" font-size="7" font-weight="500" fill="{fc}">{label}</text>
</svg>"##,
        fc = fill_color,
        label = label,
    )
}

fn render_circle_svg(usage: f64, fill_color: &str, track_color: &str) -> String {
    let radius = 7.0_f64;
    let (filled, circumference) = calculate_circle_dash_array(usage, radius);

    format!(
        r##"<svg viewBox="0 0 16 16" width="16" height="16" xmlns="http://www.w3.org/2000/svg">
  <circle cx="8" cy="8" r="{r}" fill="none" stroke-width="2" stroke="{tc}" stroke-dasharray="{c} {c}"/>
  <circle cx="8" cy="8" r="{r}" fill="none" stroke-width="2" stroke="{fc}" stroke-dasharray="{f} {c}" transform="rotate(-90 8 8)"/>
</svg>"##,
        r = radius,
        tc = track_color,
        fc = fill_color,
        c = circumference,
        f = filled,
    )
}

/// Build full SVG combining all providers
pub fn build_full_svg(
    providers: &[(String, f64, DisplayMode)],
    dark: bool,
) -> String {
    if providers.is_empty() {
        // Placeholder: gray circle
        return r##"<svg viewBox="0 0 16 16" width="16" height="16" xmlns="http://www.w3.org/2000/svg">
  <circle cx="8" cy="8" r="6" fill="none" stroke="#48484A" stroke-width="2"/>
</svg>"##.to_string();
    }

    let total_width: u32 = providers.iter().map(|(_, _, m)| m.width()).sum();
    let spacing = 4 * (providers.len() as u32 - 1);
    let width = total_width + spacing;
    let height = providers.iter().map(|(_, _, m)| m.height()).max().unwrap_or(16);

    let mut segments = Vec::new();
    let mut x_offset = 0;

    for (name, usage, mode) in providers {
        let segment_svg = render_provider_svg(name, *usage, *mode, dark);
        // Extract the inner content (without svg tags)
        let inner = segment_svg
            .strip_prefix("<svg viewBox=\"0 0 ")
            .and_then(|s| s.split("\" width=\"").nth(1))
            .and_then(|s| s.split("\" height=\"").nth(1))
            .and_then(|s| s.split("\" xmlns=\"http://www.w3.org/2000/svg\">\n").nth(1))
            .and_then(|s| s.strip_suffix("</svg>"))
            .unwrap_or("");

        segments.push(format!(
            r##"  <g transform="translate({}, 0)">{}</g>"##,
            x_offset, inner
        ));

        x_offset += mode.width() as i32 + 4;
    }

    format!(
        r##"<svg viewBox="0 0 {} {}" width="{}" height="{}" xmlns="http://www.w3.org/2000/svg">
{}</svg>"##,
        width, height, width, height,
        segments.join("\n")
    )
}

/// Render SVG to raw RGBA pixel data. Renders at 2× for Retina clarity.
pub fn render_svg_to_rgba(svg: &str, pt_width: u32, pt_height: u32) -> Option<Vec<u8>> {
    use resvg::{tiny_skia, usvg};

    let scale = 2_u32;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_circle_dash_array() {
        let (filled, circ) = calculate_circle_dash_array(0.0, 7.0);
        assert_eq!(filled, 0.0);
        assert!((circ - 43.98).abs() < 0.1);

        let (filled, _) = calculate_circle_dash_array(45.0, 7.0);
        assert!((filled - 19.79).abs() < 0.1);

        let (filled, _) = calculate_circle_dash_array(100.0, 7.0);
        assert!((filled - 43.98).abs() < 0.1);
    }

    #[test]
    fn test_render_bar_svg() {
        let svg = render_bar_svg("Test", 50.0, "50%", "#000", "#E5E5EA", "#34C759");
        assert!(svg.contains("Test: 50%"));
        assert!(svg.contains("#34C759"));
        assert!(svg.contains("viewBox=\"0 0 32 16\""));
    }

    #[test]
    fn test_render_text_svg() {
        let svg = render_text_svg("Test", "50%", "#000", "#34C759");
        assert!(svg.contains("Test: 50%"));
        assert!(svg.contains("#34C759"));
        assert!(svg.contains("viewBox=\"0 0 24 10\""));
    }

    #[test]
    fn test_render_circle_svg() {
        let svg = render_circle_svg(50.0, "#34C759", "#E5E5EA");
        assert!(svg.contains("#34C759"));
        assert!(svg.contains("viewBox=\"0 0 16 16\""));
        assert!(svg.contains("rotate(-90 8 8)"));
    }
}
