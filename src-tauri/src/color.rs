/// Returns color based on usage percentage.
/// 0-50%: Green, 51-80%: Yellow, 81-100%: Red
pub fn get_usage_color(percentage: f64) -> &'static str {
    const GREEN: &str = "#34C759";
    const YELLOW: &str = "#FFD60A";
    const RED: &str = "#FF453A";

    let pct = percentage.clamp(0.0, 100.0);
    if pct <= 50.0 {
        GREEN
    } else if pct <= 80.0 {
        YELLOW
    } else {
        RED
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_low_usage_is_green() {
        assert_eq!(get_usage_color(0.0), "#34C759");
        assert_eq!(get_usage_color(25.0), "#34C759");
        assert_eq!(get_usage_color(50.0), "#34C759");
    }

    #[test]
    fn test_medium_usage_is_yellow() {
        assert_eq!(get_usage_color(51.0), "#FFD60A");
        assert_eq!(get_usage_color(65.0), "#FFD60A");
        assert_eq!(get_usage_color(80.0), "#FFD60A");
    }

    #[test]
    fn test_high_usage_is_red() {
        assert_eq!(get_usage_color(81.0), "#FF453A");
        assert_eq!(get_usage_color(90.0), "#FF453A");
        assert_eq!(get_usage_color(100.0), "#FF453A");
    }

    #[test]
    fn test_clamping() {
        assert_eq!(get_usage_color(-10.0), "#34C759");
        assert_eq!(get_usage_color(150.0), "#FF453A");
    }
}
