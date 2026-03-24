use std::process::Command;
use serde_json;

/// Provider trait - all providers implement this
pub trait Provider {
    /// Get provider ID (e.g., "claude", "ccs")
    fn id(&self) -> &str;

    /// Get display name (e.g., "Claude 5h", "CCS 5h")
    fn display_name(&self) -> &str;

    /// Fetch current usage percentage
    fn fetch_usage(&self) -> Option<f64>;
}

/// Claude provider using Anthropic OAuth API
pub struct ClaudeProvider;

impl Provider for ClaudeProvider {
    fn id(&self) -> &str {
        "claude"
    }

    fn display_name(&self) -> &str {
        "Claude 5h"
    }

    fn fetch_usage(&self) -> Option<f64> {
        let token = Self::get_token()?;
        let resp = ureq::get("https://api.anthropic.com/api/oauth/usage")
            .set("Authorization", &format!("Bearer {token}"))
            .set("anthropic-beta", "oauth-2025-04-20")
            .call()
            .ok()?;
        let data: serde_json::Value = resp.into_json().ok()?;
        data["five_hour"]["utilization"].as_f64()
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

/// CCS/GLM provider using Z.AI API
pub struct CcsProvider;

impl Provider for CcsProvider {
    fn id(&self) -> &str {
        "ccs"
    }

    fn display_name(&self) -> &str {
        "CCS 5h"
    }

    fn fetch_usage(&self) -> Option<f64> {
        let key = Self::get_api_key()?;
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
}

impl CcsProvider {
    fn get_api_key() -> Option<String> {
        use std::path::PathBuf;
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

/// Get all available providers
pub fn all_providers() -> Vec<Box<dyn Provider + Send + Sync>> {
    vec![
        Box::new(ClaudeProvider),
        Box::new(CcsProvider),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_ids() {
        assert_eq!(ClaudeProvider.id(), "claude");
        assert_eq!(CcsProvider.id(), "ccs");
    }

    #[test]
    fn test_display_names() {
        assert_eq!(ClaudeProvider.display_name(), "Claude 5h");
        assert_eq!(CcsProvider.display_name(), "CCS 5h");
    }

    #[test]
    fn test_all_providers_count() {
        let providers = all_providers();
        assert_eq!(providers.len(), 2);
    }
}
