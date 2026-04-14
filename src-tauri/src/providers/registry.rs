use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderAdapter {
    pub provider_id: &'static str,
    pub display_name: &'static str,
    pub default_models: &'static [&'static str],
    pub requires_session_secret: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ProviderRegistry {
    adapters: BTreeMap<&'static str, ProviderAdapter>,
}

impl ProviderRegistry {
    pub fn register(&mut self, adapter: ProviderAdapter) {
        self.adapters.insert(adapter.provider_id, adapter);
    }

    pub fn adapter(&self, provider_id: &str) -> Option<&ProviderAdapter> {
        self.adapters.get(provider_id)
    }

    pub fn provider_ids(&self) -> Vec<&'static str> {
        self.adapters.keys().copied().collect()
    }
}

pub fn default_provider_registry() -> ProviderRegistry {
    let mut registry = ProviderRegistry::default();
    registry.register(crate::providers::generic_openai::provider_adapter());
    registry.register(crate::providers::official::provider_adapter());
    registry
}
