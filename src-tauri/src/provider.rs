use std::process::Command;
use std::path::PathBuf;
use serde_json;

/// Usage data for a single time window
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UsageWindow {
    pub utilization: f64,      // 0-100
    pub resets_at: String,     // ISO timestamp
}

/// Named usage window for menu rendering
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UsageBucket {
    pub id: String,
    pub label: String,
    pub utilization: f64,
    pub resets_at: String,
}

/// Complete usage data for a provider
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProviderUsage {
    pub provider: String,
    pub label: String,
    pub usage_windows: Vec<UsageBucket>,
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
            usage_windows: Vec::new(),
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

fn usage_bucket(id: &str, label: &str, window: &UsageWindow) -> UsageBucket {
    UsageBucket {
        id: id.to_string(),
        label: label.to_string(),
        utilization: window.utilization,
        resets_at: window.resets_at.clone(),
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

        let five_hour = map_window("five_hour");
        let seven_day = map_window("seven_day");
        let seven_day_opus = map_window("seven_day_opus");
        let seven_day_sonnet = map_window("seven_day_sonnet");

        let mut usage_windows = Vec::new();
        if let Some(window) = &five_hour {
            usage_windows.push(usage_bucket("five_hour", "5h", window));
        }
        if let Some(window) = &seven_day {
            usage_windows.push(usage_bucket("seven_day", "7d", window));
        }
        if let Some(window) = &seven_day_opus {
            usage_windows.push(usage_bucket("seven_day_opus", "Opus", window));
        }
        if let Some(window) = &seven_day_sonnet {
            usage_windows.push(usage_bucket("seven_day_sonnet", "Sonnet", window));
        }

        Ok(ProviderUsage {
            provider: "claude".to_string(),
            label: "Claude".to_string(),
            usage_windows,
            five_hour,
            seven_day,
            seven_day_opus,
            seven_day_sonnet,
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
        "zAI"
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

        let limits = data["data"]["limits"]
            .as_array()
            .ok_or_else(|| "Missing usage limits".to_string())?;

        let map_limit = |limit: &serde_json::Value| -> Option<UsageWindow> {
            Some(UsageWindow {
                utilization: limit.get("percentage")?.as_f64()?,
                resets_at: limit.get("nextResetTime")?.as_i64()?.to_string(),
            })
        };

        let five_hour = limits.iter()
            .find(|l| l["type"].as_str() == Some("TOKENS_LIMIT"))
            .and_then(map_limit);
        let thirty_day = limits.iter()
            .find(|l| l["type"].as_str() == Some("TIME_LIMIT"))
            .and_then(map_limit);

        let mut usage_windows = Vec::new();
        if let Some(window) = &five_hour {
            usage_windows.push(usage_bucket("five_hour", "5h", window));
        }
        if let Some(window) = &thirty_day {
            usage_windows.push(usage_bucket("thirty_day", "30d", window));
        }

        Ok(ProviderUsage {
            provider: "glm".to_string(),
            label: "zAI".to_string(),
            usage_windows,
            five_hour,
            seven_day: thirty_day,
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
        let auth = Self::get_auth()
            .ok_or_else(|| "Not logged in — run `codex` to authenticate".to_string())?;

        let output = Command::new("curl")
            .args([
                "-sS",
                "--http2",
                "--max-time", "10",
                "--write-out", "\n%{http_code}",
                "https://chatgpt.com/backend-api/wham/usage",
                "-H", &format!("Authorization: Bearer {}", auth.access_token),
                "-H", &format!("ChatGPT-Account-Id: {}", auth.account_id),
                "-H", "User-Agent: codex-cli",
                "-H", "Accept: application/json",
            ])
            .output()
            .map_err(|e| format!("Network error: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if stderr.is_empty() {
                "Network error".to_string()
            } else {
                format!("Network error: {}", stderr)
            });
        }

        let raw = String::from_utf8(output.stdout)
            .map_err(|e| format!("Invalid response encoding: {}", e))?;
        let (body, status) = raw
            .rsplit_once('\n')
            .ok_or_else(|| "Invalid response format".to_string())?;
        let status: u16 = status.trim().parse()
            .map_err(|_| "Invalid HTTP status".to_string())?;

        if status == 401 {
            return Err("Token expired — re-authenticate with `codex`".to_string());
        }
        if status == 403 {
            return Err("Access denied — check your Codex plan".to_string());
        }
        if status == 429 {
            return Err("Too many requests — try again later".to_string());
        }
        if status >= 500 {
            return Err(format!("OpenAI API error ({})", status));
        }
        if status >= 400 {
            return Err(format!("HTTP {}", status));
        }

        let data: serde_json::Value = serde_json::from_str(body)
            .map_err(|e| format!("Invalid response: {}", e))?;
        let rate_limit = data.get("rate_limit")
            .ok_or_else(|| "Missing rate_limit".to_string())?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let map_window = |window: Option<&serde_json::Value>| -> Option<UsageWindow> {
            let w = window?;
            Some(UsageWindow {
                utilization: w.get("used_percent")?.as_f64()?,
                resets_at: w.get("reset_at")?.as_i64()?.to_string(),
            })
        };

        let five_hour = map_window(rate_limit.get("primary_window"));
        let seven_day = map_window(rate_limit.get("secondary_window"));

        let mut usage_windows = Vec::new();
        if let Some(window) = &five_hour {
            usage_windows.push(usage_bucket("five_hour", "5h", window));
        }
        if let Some(window) = &seven_day {
            usage_windows.push(usage_bucket("seven_day", "7d", window));
        }

        Ok(ProviderUsage {
            provider: "codex".to_string(),
            label: "Codex".to_string(),
            usage_windows,
            five_hour,
            seven_day,
            seven_day_opus: None,
            seven_day_sonnet: None,
            error: None,
            fetched_at: now,
        })
    }
}

struct CodexAuth {
    access_token: String,
    account_id: String,
}

impl CodexProvider {
    fn get_auth() -> Option<CodexAuth> {
        let path = PathBuf::from(std::env::var("HOME").ok()?)
            .join(".codex")
            .join("auth.json");
        let content = std::fs::read_to_string(path).ok()?;
        let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
        let tokens = parsed.get("tokens")?;
        Some(CodexAuth {
            access_token: tokens.get("access_token")?.as_str()?.to_string(),
            account_id: tokens.get("account_id")?.as_str()?.to_string(),
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
        assert_eq!(GlmProvider.display_name(), "zAI");
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
        assert!(usage.usage_windows.is_empty());
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
