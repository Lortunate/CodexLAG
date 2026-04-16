pub mod callback;
pub mod openai;
pub mod openai_claims;
pub mod session_store;

use crate::models::{ProviderAuthProfile, ProviderDescriptor};
use crate::providers::generic_openai::GENERIC_OPENAI_PROVIDER_ID;
use crate::providers::registry::default_provider_registry;

pub fn list_provider_descriptors() -> Vec<ProviderDescriptor> {
    let registry = default_provider_registry();
    let mut descriptors = registry
        .provider_ids()
        .into_iter()
        .filter_map(|provider_id| {
            let adapter = registry.adapter(provider_id)?;
            Some(ProviderDescriptor {
                provider_id: adapter.provider_id().to_string(),
                auth_profile: if adapter.supports_browser_login() {
                    ProviderAuthProfile::BrowserOauthPkce
                } else {
                    ProviderAuthProfile::StaticApiKey
                },
                supports_model_discovery: adapter.provider_id() == GENERIC_OPENAI_PROVIDER_ID,
                supports_capability_probe: adapter.supports_balance(),
            })
        })
        .collect::<Vec<_>>();
    descriptors.sort_by(|left, right| left.provider_id.cmp(&right.provider_id));
    descriptors
}
