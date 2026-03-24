use std::process::Command;
use std::path::PathBuf;
use serde_json;

/// Usage data for a single time window
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UsageWindow {
    pub utilization: f64,      // 0-100
    pub resets_at: String,     // ISO timestamp
}

/// Complete usage data for a provider
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProviderUsage {
    pub provider: String,
    pub label: String,
    pub five_hour: Option<UsageWindow>,
    pub seven_day: Option<UsageWindow>,
    pub seven_day_opus: Option<UsageWindow>,
    pub seven_day_sonnet: Option<UsageWindow>,
    pub error: Option<String>,
    pub fetched_at: i64,  // Unix timestamp
}

impl ProviderUsage {
    pub fn error(provider: &str, label: &str, error: String) -> Self {
        Self {
            provider: provider.to_string(),
            label: label.to_string(),
            five_hour: None,
            seven_day: None,
            seven_day_opus: None,
            seven_day_sonnet: None,
            error: Some(error),
            fetched_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        }
    }
}

/// Provider trait - all providers implement this
pub trait Provider {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn fetch_usage_data(&self) -> Result<ProviderUsage, String>;

    /// Legacy method for backward compatibility
    #[deprecated(note = "Use fetch_usage_data instead")]
    fn fetch_usage(&self) -> Option<f64> {
        self.fetch_usage_data()
            .ok()
            .and_then(|u| u.five_hour)
            .map(|w| w.utilization)
    }
}

/// Claude provider using Anthropic OAuth API
pub struct ClaudeProvider;

impl Provider for ClaudeProvider {
    fn id(&self) -> &str {
        "claude"
    }

    fn display_name(&self) -> &str {
        "Claude"
    }

    fn fetch_usage_data(&self) -> Result<ProviderUsage, String> {
        let token = Self::get_token()
            .ok_or_else(|| "Not logged in — run `claude` to authenticate".to_string())?;

        let resp = ureq::get("https://api.anthropic.com/api/oauth/usage")
            .set("Authorization", &format!("Bearer {token}"))
            .set("anthropic-beta", "oauth-2025-04-20")
            .timeout(std::time::Duration::from_secs(10))
            .call()
            .map_err(|e| format!("Network error: {}", e))?;

        if resp.status() == 401 {
            return Err("Token expired — re-authenticate with `claude`".to_string());
        }
        if resp.status() == 429 {
            return Err("Too many requests — try again later".to_string());
        }
        if resp.status() >= 500 {
            return Err(format!("API error ({})", resp.status()));
        }

        let data: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("Invalid response: {}", e))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let map_window = |key: &str| -> Option<UsageWindow> {
            let w = data.get(key)?;
            Some(UsageWindow {
                utilization: w.get("utilization")?.as_f64()?,
                resets_at: w.get("resets_at")?.as_str()?.to_string(),
            })
        };

        Ok(ProviderUsage {
            provider: "claude".to_string(),
            label: "Claude".to_string(),
            five_hour: map_window("five_hour"),
            seven_day: map_window("seven_day"),
            seven_day_opus: map_window("seven_day_opus"),
            seven_day_sonnet: map_window("seven_day_sonnet"),
            error: None,
            fetched_at: now,
        })
    }
}

impl ClaudeProvider {
    fn get_token() -> Option<String> {
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
}

/// GLM provider using Z.AI API
pub struct GlmProvider;

impl Provider for GlmProvider {
    fn id(&self) -> &str {
        "glm"
    }

    fn display_name(&self) -> &str {
        "GLM (Z.AI)"
    }

    fn fetch_usage_data(&self) -> Result<ProviderUsage, String> {
        let key = Self::get_api_key()
            .ok_or_else(|| "GLM API key not found in ~/.ccs/glm.settings.json".to_string())?;

        let resp = ureq::get("https://api.z.ai/api/monitor/usage/quota/limit")
            .set("Authorization", &format!("Bearer {key}"))
            .timeout(std::time::Duration::from_secs(10))
            .call()
            .map_err(|e| format!("Network error: {}", e))?;

        if resp.status() == 401 {
            return Err("GLM API key invalid or expired".to_string());
        }
        if resp.status() >= 500 {
            return Err(format!("GLM API error ({})", resp.status()));
        }

        let data: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("Invalid response: {}", e))?;

        if data["success"].as_bool() != Some(true) {
            return Err("API returned success=false".to_string());
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let utilization = data["data"]["limits"]
            .as_array()
            .and_then(|arr| {
                arr.iter()
                    .find(|l| l["type"].as_str() == Some("TOKENS_LIMIT"))
                    .and_then(|l| l["percentage"].as_f64())
            })
            .unwrap_or(0.0);

        Ok(ProviderUsage {
            provider: "glm".to_string(),
            label: "GLM (Z.AI)".to_string(),
            five_hour: Some(UsageWindow {
                utilization,
                resets_at: "N/A".to_string(),
            }),
            seven_day: None,
            seven_day_opus: None,
            seven_day_sonnet: None,
            error: None,
            fetched_at: now,
        })
    }
}

impl GlmProvider {
    fn get_api_key() -> Option<String> {
        let path = PathBuf::from(std::env::var("HOME").ok()?)
            .join(".ccs")
            .join("glm.settings.json");
        let content = std::fs::read_to_string(path).ok()?;
        let s: serde_json::Value = serde_json::from_str(&content).ok()?;
        s["env"]["ANTHROPIC_AUTH_TOKEN"]
            .as_str()
            .or_else(|| s["env"]["GLM_API_KEY"].as_str())
            .or_else(|| s["ANTHROPIC_AUTH_TOKEN"].as_str())
            .or_else(|| s["GLM_API_KEY"].as_str())
            .map(|v| v.to_string())
    }
}

/// Codex provider placeholder
pub struct CodexProvider;

impl Provider for CodexProvider {
    fn id(&self) -> &str {
        "codex"
    }

    fn display_name(&self) -> &str {
        "Codex"
    }

    fn fetch_usage_data(&self) -> Result<ProviderUsage, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Ok(ProviderUsage {
            provider: "codex".to_string(),
            label: "Codex".to_string(),
            five_hour: None,
            seven_day: None,
            seven_day_opus: None,
            seven_day_sonnet: None,
            error: Some("Not implemented yet".to_string()),
            fetched_at: now,
        })
    }
}


/// Get all available providers
pub fn all_providers() -> Vec<Box<dyn Provider + Send + Sync>> {
    vec![
        Box::new(ClaudeProvider),
        Box::new(GlmProvider),
        Box::new(CodexProvider),
    ]
}

pub fn fetch_all_usage() -> Vec<ProviderUsage> {
    all_providers()
        .into_iter()
        .map(|p| {
            p.fetch_usage_data()
                .unwrap_or_else(|e| ProviderUsage::error(p.id(), p.display_name(), e))
        })
        .collect()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_ids() {
        assert_eq!(ClaudeProvider.id(), "claude");
        assert_eq!(GlmProvider.id(), "glm");
        assert_eq!(CodexProvider.id(), "codex");
    }

    #[test]
    fn test_display_names() {
        assert_eq!(ClaudeProvider.display_name(), "Claude");
        assert_eq!(GlmProvider.display_name(), "GLM (Z.AI)");
        assert_eq!(CodexProvider.display_name(), "Codex");
    }

    #[test]
    fn test_all_providers_count() {
        let providers = all_providers();
        assert_eq!(providers.len(), 3);
    }

    #[test]
    fn test_provider_usage_error() {
        let usage = ProviderUsage::error("test", "Test Provider", "Test error".to_string());
        assert_eq!(usage.provider, "test");
        assert_eq!(usage.label, "Test Provider");
        assert_eq!(usage.error, Some("Test error".to_string()));
        assert!(usage.five_hour.is_none());
        assert!(usage.seven_day.is_none());
    }

    #[test]
    fn test_fetch_all_usage() {
        let results = fetch_all_usage();
        // Should return 3 results even if some fail
        assert_eq!(results.len(), 3);
        // Check that all have provider and label
        for r in &results {
            assert!(!r.provider.is_empty());
            assert!(!r.label.is_empty());
            assert!(r.fetched_at > 0);
        }
    }
}
