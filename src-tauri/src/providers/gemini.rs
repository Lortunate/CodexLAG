use crate::providers::registry::ProviderAdapter;

pub const GEMINI_PROVIDER_ID: &str = "gemini_official";
pub const GEMINI_DEFAULT_MODELS: &[&str] = &["gemini-2.5-flash"];

#[derive(Debug)]
pub struct GeminiOfficialAdapter;

impl ProviderAdapter for GeminiOfficialAdapter {
    fn provider_id(&self) -> &'static str {
        GEMINI_PROVIDER_ID
    }

    fn supports_browser_login(&self) -> bool {
        false
    }

    fn supports_balance(&self) -> bool {
        false
    }
}

static GEMINI_OFFICIAL_ADAPTER: GeminiOfficialAdapter = GeminiOfficialAdapter;

pub fn provider_adapter() -> &'static dyn ProviderAdapter {
    &GEMINI_OFFICIAL_ADAPTER
}
