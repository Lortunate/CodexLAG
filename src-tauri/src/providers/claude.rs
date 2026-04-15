use crate::providers::registry::ProviderAdapter;

pub const CLAUDE_PROVIDER_ID: &str = "claude_official";
pub const CLAUDE_DEFAULT_MODELS: &[&str] = &["claude-3-7-sonnet"];

#[derive(Debug)]
pub struct ClaudeOfficialAdapter;

impl ProviderAdapter for ClaudeOfficialAdapter {
    fn provider_id(&self) -> &'static str {
        CLAUDE_PROVIDER_ID
    }

    fn supports_browser_login(&self) -> bool {
        false
    }

    fn supports_balance(&self) -> bool {
        false
    }
}

static CLAUDE_OFFICIAL_ADAPTER: ClaudeOfficialAdapter = ClaudeOfficialAdapter;

pub fn provider_adapter() -> &'static dyn ProviderAdapter {
    &CLAUDE_OFFICIAL_ADAPTER
}
