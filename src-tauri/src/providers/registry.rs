use std::collections::BTreeMap;

use crate::providers::claude::CLAUDE_PROVIDER_ID;
use crate::providers::generic_openai::GENERIC_OPENAI_PROVIDER_ID;
use crate::providers::gemini::GEMINI_PROVIDER_ID;
use crate::providers::official::OFFICIAL_OPENAI_PROVIDER_ID;

pub trait ProviderAdapter: std::fmt::Debug + Send + Sync {
    fn provider_id(&self) -> &'static str;
    fn supports_browser_login(&self) -> bool;
    fn supports_balance(&self) -> bool;
}

#[derive(Debug, Default)]
pub struct ProviderRegistry {
    adapters: BTreeMap<&'static str, &'static dyn ProviderAdapter>,
}

impl ProviderRegistry {
    pub fn register(&mut self, adapter: &'static dyn ProviderAdapter) {
        self.adapters.insert(adapter.provider_id(), adapter);
    }

    pub fn adapter(&self, provider_id: &str) -> Option<&'static dyn ProviderAdapter> {
        let canonical = match provider_id {
            "claude" | "anthropic" => CLAUDE_PROVIDER_ID,
            "gemini" => GEMINI_PROVIDER_ID,
            "openai" => OFFICIAL_OPENAI_PROVIDER_ID,
            "generic_openai" => GENERIC_OPENAI_PROVIDER_ID,
            other => other,
        };
        self.adapters.get(canonical).copied()
    }

    pub fn provider_ids(&self) -> Vec<&'static str> {
        self.adapters.keys().copied().collect()
    }
}

pub fn default_provider_registry() -> ProviderRegistry {
    let mut registry = ProviderRegistry::default();
    registry.register(crate::providers::claude::provider_adapter());
    registry.register(crate::providers::generic_openai::provider_adapter());
    registry.register(crate::providers::gemini::provider_adapter());
    registry.register(crate::providers::official::provider_adapter());
    registry
}
