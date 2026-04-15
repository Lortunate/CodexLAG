use crate::auth::session_store::ProviderSessionStore;
use crate::models::FeatureCapability;
use crate::providers::capabilities::feature_capabilities_for_model_ids;
use crate::providers::generic_openai::{
    generic_openai_inventory_models, parse_generic_openai_config, GENERIC_OPENAI_PROVIDER_ID,
};
use crate::providers::registry::{default_provider_registry, ProviderAdapter};
use crate::secret_store::SecretKey;
use crate::state::AppState;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProviderInventorySummary {
    pub providers: Vec<ProviderInventoryEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderInventoryEntry {
    pub provider_id: String,
    pub endpoint_id: String,
    pub display_name: String,
    pub available: bool,
    pub registered: bool,
    pub base_url: Option<String>,
    pub model_ids: Vec<String>,
    pub feature_capabilities: Vec<FeatureCapability>,
}

pub fn project_provider_inventory_summary(state: &AppState) -> ProviderInventorySummary {
    let registry = default_provider_registry();
    let mut providers: Vec<_> = state
        .iter_imported_official_accounts()
        .map(|account| {
            let adapter = registry.adapter(account.provider.as_str()).copied();
            let token_secret = state
                .secret(&SecretKey::new(account.token_credential_ref.clone()))
                .ok();
            let has_session_secret = state
                .secret(&SecretKey::new(account.session_credential_ref.clone()))
                .is_ok();

            let (model_ids, base_url) = inventory_models_for_account(
                adapter,
                account.provider.as_str(),
                token_secret.as_deref(),
            );
            let available = adapter.is_some()
                && account.session.status == "active"
                && token_secret.is_some()
                && adapter
                    .is_none_or(|adapter| !adapter.requires_session_secret || has_session_secret);

            ProviderInventoryEntry {
                provider_id: adapter
                    .map(|adapter| adapter.provider_id.to_string())
                    .unwrap_or_else(|| account.provider.clone()),
                endpoint_id: account.account_id.clone(),
                display_name: account.name.clone(),
                available,
                registered: adapter.is_some(),
                base_url,
                feature_capabilities: feature_capabilities_for_model_ids(&model_ids),
                model_ids,
            }
        })
        .collect();
    providers.extend(state.iter_provider_sessions().map(|session| {
        let adapter = registry.adapter(session.provider_id.as_str()).copied();
        let stored = ProviderSessionStore::load(
            state,
            session.provider_id.as_str(),
            session.account_id.as_str(),
        )
        .ok()
        .flatten();
        let token_secret = stored.as_ref().map(|stored| stored.token_secret.as_str());
        let has_session_secret = stored
            .as_ref()
            .map(|stored| !stored.session_secret.trim().is_empty())
            .unwrap_or(false);
        let (model_ids, base_url) =
            inventory_models_for_account(adapter, session.provider_id.as_str(), token_secret);
        let available = adapter.is_some()
            && session.auth_state == "active"
            && token_secret.is_some()
            && adapter
                .is_none_or(|adapter| !adapter.requires_session_secret || has_session_secret);

        ProviderInventoryEntry {
            provider_id: adapter
                .map(|adapter| adapter.provider_id.to_string())
                .unwrap_or_else(|| session.provider_id.clone()),
            endpoint_id: session.account_id.clone(),
            display_name: session.display_name.clone(),
            available,
            registered: adapter.is_some(),
            base_url,
            feature_capabilities: feature_capabilities_for_model_ids(&model_ids),
            model_ids,
        }
    }));
    providers.sort_by(|left, right| {
        left.display_name
            .cmp(&right.display_name)
            .then_with(|| left.endpoint_id.cmp(&right.endpoint_id))
    });

    ProviderInventorySummary { providers }
}

fn inventory_models_for_account(
    adapter: Option<ProviderAdapter>,
    provider_id: &str,
    token_secret: Option<&str>,
) -> (Vec<String>, Option<String>) {
    if matches!(provider_id, GENERIC_OPENAI_PROVIDER_ID | "generic_openai") {
        if let Some(token_secret) = token_secret {
            if let Ok(config) = parse_generic_openai_config(token_secret) {
                return (
                    generic_openai_inventory_models(&config),
                    Some(config.base_url),
                );
            }
        }
    }

    let model_ids = adapter
        .map(|adapter| {
            adapter
                .default_models
                .iter()
                .map(|model| (*model).to_string())
                .collect()
        })
        .unwrap_or_default();
    (model_ids, None)
}
