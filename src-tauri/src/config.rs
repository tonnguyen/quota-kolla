use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Display mode for a provider
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DisplayMode {
    Bar,
    Text,
    Circle,
}

impl Default for DisplayMode {
    fn default() -> Self {
        DisplayMode::Bar
    }
}

impl DisplayMode {
    /// Parse from string, defaulting to Bar for invalid values
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "text" => DisplayMode::Text,
            "circle" => DisplayMode::Circle,
            _ => DisplayMode::Bar,
        }
    }

    /// Get width in points for this mode
    pub fn width(&self) -> u32 {
        match self {
            DisplayMode::Bar => 32,
            DisplayMode::Text => 24,
            DisplayMode::Circle => 16,
        }
    }

    /// Get height in points for this mode
    pub fn height(&self) -> u32 {
        match self {
            DisplayMode::Bar => 16,
            DisplayMode::Text => 10,
            DisplayMode::Circle => 16,
        }
    }
}

/// Configuration for a single provider
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderConfig {
    pub visible: bool,
    pub mode: String,
}

impl ProviderConfig {
    pub fn get_mode(&self) -> DisplayMode {
        DisplayMode::from_str(&self.mode)
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        ProviderConfig {
            visible: true,
            mode: "bar".to_string(),
        }
    }
}

/// Top-level configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub version: u32,
    pub providers: HashMap<String, ProviderConfig>,
}

impl Default for Config {
    fn default() -> Self {
        let mut providers = HashMap::new();
        providers.insert("claude".to_string(), ProviderConfig::default());
        providers.insert("ccs".to_string(), ProviderConfig::default());

        Config {
            version: 1,
            providers,
        }
    }
}

impl Config {
    /// Get config directory path
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("menubar-progress")
    }

    /// Get config file path
    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    /// Load config from file, or create default if missing
    pub fn load() -> Self {
        let path = Self::config_path();

        if !path.exists() {
            let default = Self::default();
            default.save().unwrap_or_else(|e| {
                eprintln!("Failed to save default config: {}", e);
            });
            return default;
        }

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to read config: {}", e);
                return Self::default();
            }
        };

        let config: Config = serde_json::from_str(&content)
            .unwrap_or_else(|e| {
                eprintln!("Failed to parse config: {}, using default", e);
                Self::default()
            });

        // Validate provider keys and warn about unknown ones
        config.validate_provider_keys();

        config
    }

    /// Validate provider keys and warn about unknown providers
    fn validate_provider_keys(&self) {
        let known_providers = ["claude", "ccs"];
        for (key, _) in &self.providers {
            if !known_providers.contains(&key.as_str()) {
                eprintln!("Warning: Unknown provider '{}' in config. Known providers: claude, ccs", key);
            }
        }
    }

    /// Save config to file
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path();

        // Ensure config directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        fs::write(&path, json)?;
        Ok(())
    }

    /// Get list of visible provider IDs
    pub fn visible_providers(&self) -> Vec<String> {
        self.providers
            .iter()
            .filter(|(_, cfg)| cfg.visible)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Calculate total width for visible providers
    pub fn total_width(&self) -> u32 {
        let visible: Vec<_> = self
            .providers
            .iter()
            .filter(|(_, cfg)| cfg.visible)
            .collect();

        if visible.is_empty() {
            return 16; // Placeholder width
        }

        let provider_width: u32 = visible
            .iter()
            .map(|(_, cfg)| cfg.get_mode().width())
            .sum();

        let spacing = if visible.len() > 1 { 4 } else { 0 };
        provider_width + spacing * (visible.len() as u32 - 1)
    }

    /// Get maximum height among visible providers
    pub fn max_height(&self) -> u32 {
        self.providers
            .iter()
            .filter(|(_, cfg)| cfg.visible)
            .map(|(_, cfg)| cfg.get_mode().height())
            .max()
            .unwrap_or(16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_mode_from_str() {
        assert_eq!(DisplayMode::from_str("bar"), DisplayMode::Bar);
        assert_eq!(DisplayMode::from_str("Bar"), DisplayMode::Bar);
        assert_eq!(DisplayMode::from_str("text"), DisplayMode::Text);
        assert_eq!(DisplayMode::from_str("circle"), DisplayMode::Circle);
        assert_eq!(DisplayMode::from_str("invalid"), DisplayMode::Bar);
    }

    #[test]
    fn test_display_mode_dimensions() {
        assert_eq!(DisplayMode::Bar.width(), 32);
        assert_eq!(DisplayMode::Bar.height(), 16);
        assert_eq!(DisplayMode::Text.width(), 24);
        assert_eq!(DisplayMode::Text.height(), 10);
        assert_eq!(DisplayMode::Circle.width(), 16);
        assert_eq!(DisplayMode::Circle.height(), 16);
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.version, 1);
        assert!(config.providers.contains_key("claude"));
        assert!(config.providers.contains_key("ccs"));
        assert_eq!(config.providers.len(), 2);
    }

    #[test]
    fn test_visible_providers() {
        let mut config = Config::default();
        config.providers.get_mut("ccs").unwrap().visible = false;

        let visible = config.visible_providers();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0], "claude");
    }

    #[test]
    fn test_total_width() {
        let mut config = Config::default();
        // Both visible, both bar mode: 32 + 4 + 32 = 68
        assert_eq!(config.total_width(), 68);

        // Hide CCS: 32
        config.providers.get_mut("ccs").unwrap().visible = false;
        assert_eq!(config.total_width(), 32);

        // Both hidden: placeholder 16
        config.providers.get_mut("claude").unwrap().visible = false;
        assert_eq!(config.total_width(), 16);
    }

    #[test]
    fn test_unknown_provider_ignored() {
        // Config with unknown provider should still load
        // Unknown providers are logged but don't cause failure
        let mut config = Config::default();
        config.providers.insert("unknown".to_string(), ProviderConfig::default());
        assert_eq!(config.providers.len(), 3);
        // validate_provider_keys would warn but not remove the entry
    }
}
