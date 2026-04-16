use crate::auth::session_store::ProviderSessionStore;
use crate::providers::claude::{CLAUDE_DEFAULT_MODELS, CLAUDE_PROVIDER_ID};
use crate::providers::gemini::{GEMINI_DEFAULT_MODELS, GEMINI_PROVIDER_ID};
use crate::providers::generic_openai::{
    generic_openai_inventory_models, parse_generic_openai_config, GENERIC_OPENAI_DEFAULT_MODELS,
    GENERIC_OPENAI_PROVIDER_ID,
};
use crate::providers::official::{
    official_entitlement_from_token_secret, OfficialEntitlement, OFFICIAL_DEFAULT_MODELS,
    OFFICIAL_OPENAI_PROVIDER_ID,
};
use crate::providers::registry::default_provider_registry;
use crate::secret_store::SecretKey;
use crate::state::AppState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderInventorySummary {
    pub accounts: Vec<ProviderAccountSummary>,
    pub models: Vec<ModelCapabilitySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderAccountSummary {
    pub provider_id: String,
    pub account_id: String,
    pub display_name: String,
    pub auth_state: String,
    pub status: Option<String>,
    pub available: bool,
    pub registered: bool,
    pub base_url: Option<String>,
    pub plan_type: Option<String>,
    pub subscription_active_start: Option<String>,
    pub subscription_active_until: Option<String>,
    pub claim_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelCapabilitySummary {
    pub provider_id: String,
    pub account_id: String,
    pub model_id: String,
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub supports_reasoning: bool,
    pub source: String,
}

struct InventoryProjection {
    account: ProviderAccountSummary,
    models: Vec<ModelCapabilitySummary>,
}

pub fn project_provider_inventory_summary(state: &AppState) -> ProviderInventorySummary {
    let registry = default_provider_registry();
    let mut projections: Vec<_> = state
        .iter_imported_official_accounts()
        .map(|account| {
            let adapter = registry.adapter(account.provider.as_str());
            let canonical_provider_id = adapter
                .map(|adapter| adapter.provider_id())
                .unwrap_or(account.provider.as_str());
            let token_secret = state
                .secret(&SecretKey::new(account.token_credential_ref.clone()))
                .ok();
            let has_session_secret = state
                .secret(&SecretKey::new(account.session_credential_ref.clone()))
                .is_ok();
            let model_set =
                inventory_models_for_account(canonical_provider_id, token_secret.as_deref());
            let available = adapter.is_some()
                && account.session.status == "active"
                && token_secret.is_some()
                && (!requires_session_secret(canonical_provider_id) || has_session_secret);
            let entitlement =
                entitlement_for_account(canonical_provider_id, token_secret.as_deref());
            let status = inventory_status(
                account.session.status.as_str(),
                available,
                adapter.is_some(),
                token_secret.is_some(),
            );

            project_account_inventory(
                canonical_provider_id,
                account.account_id.clone(),
                account.name.clone(),
                account.session.status.clone(),
                status,
                available,
                adapter.is_some(),
                model_set.base_url,
                entitlement,
                model_set.model_ids,
                model_set.source,
            )
        })
        .collect();
    projections.extend(state.iter_provider_sessions().map(|session| {
        let adapter = registry.adapter(session.provider_id.as_str());
        let canonical_provider_id = adapter
            .map(|adapter| adapter.provider_id())
            .unwrap_or(session.provider_id.as_str());
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
        let model_set = inventory_models_for_account(canonical_provider_id, token_secret);
        let available = adapter.is_some()
            && session.auth_state == "active"
            && token_secret.is_some()
            && (!requires_session_secret(canonical_provider_id) || has_session_secret);
        let entitlement = entitlement_for_account(canonical_provider_id, token_secret);
        let status = inventory_status(
            session.auth_state.as_str(),
            available,
            adapter.is_some(),
            token_secret.is_some(),
        );

        project_account_inventory(
            canonical_provider_id,
            session.account_id.clone(),
            session.display_name.clone(),
            session.auth_state.clone(),
            status,
            available,
            adapter.is_some(),
            model_set.base_url,
            entitlement,
            model_set.model_ids,
            if model_set.source == "default" {
                "session".into()
            } else {
                model_set.source
            },
        )
    }));
    projections.sort_by(|left, right| {
        left.account
            .display_name
            .cmp(&right.account.display_name)
            .then_with(|| left.account.account_id.cmp(&right.account.account_id))
    });

    let mut accounts = Vec::with_capacity(projections.len());
    let mut models = Vec::new();
    for projection in projections {
        accounts.push(projection.account);
        models.extend(projection.models);
    }
    models.sort_by(|left, right| {
        left.provider_id
            .cmp(&right.provider_id)
            .then_with(|| left.account_id.cmp(&right.account_id))
            .then_with(|| left.model_id.cmp(&right.model_id))
    });

    ProviderInventorySummary { accounts, models }
}

struct InventoryModelSet {
    model_ids: Vec<String>,
    base_url: Option<String>,
    source: String,
}

fn inventory_models_for_account(
    provider_id: &str,
    token_secret: Option<&str>,
) -> InventoryModelSet {
    if matches!(provider_id, GENERIC_OPENAI_PROVIDER_ID | "generic_openai") {
        if let Some(token_secret) = token_secret {
            if let Ok(config) = parse_generic_openai_config(token_secret) {
                let source = if config.manual_models.is_empty() {
                    "discovery"
                } else {
                    "manual"
                };
                return InventoryModelSet {
                    model_ids: generic_openai_inventory_models(&config),
                    base_url: Some(config.base_url),
                    source: source.into(),
                };
            }
        }
    }

    let model_ids = default_models_for_provider(provider_id)
        .iter()
        .map(|model| (*model).to_string())
        .collect();
    InventoryModelSet {
        model_ids,
        base_url: None,
        source: "default".into(),
    }
}

fn default_models_for_provider(provider_id: &str) -> &'static [&'static str] {
    match provider_id {
        CLAUDE_PROVIDER_ID | "claude" | "anthropic" => CLAUDE_DEFAULT_MODELS,
        GEMINI_PROVIDER_ID | "gemini" => GEMINI_DEFAULT_MODELS,
        OFFICIAL_OPENAI_PROVIDER_ID | "openai" => OFFICIAL_DEFAULT_MODELS,
        GENERIC_OPENAI_PROVIDER_ID | "generic_openai" => GENERIC_OPENAI_DEFAULT_MODELS,
        _ => &[],
    }
}

fn requires_session_secret(provider_id: &str) -> bool {
    matches!(provider_id, OFFICIAL_OPENAI_PROVIDER_ID | "openai")
}

fn entitlement_for_account(provider_id: &str, token_secret: Option<&str>) -> OfficialEntitlement {
    if matches!(provider_id, OFFICIAL_OPENAI_PROVIDER_ID | "openai") {
        token_secret
            .map(official_entitlement_from_token_secret)
            .unwrap_or_default()
    } else {
        OfficialEntitlement::default()
    }
}

fn inventory_status(
    auth_state: &str,
    available: bool,
    registered: bool,
    has_token_secret: bool,
) -> Option<String> {
    if !registered || !has_token_secret {
        return Some("unavailable".into());
    }
    if available {
        return Some("active".into());
    }
    if auth_state == "active" {
        Some("degraded".into())
    } else {
        Some(auth_state.to_string())
    }
}

fn project_account_inventory(
    provider_id: &str,
    account_id: String,
    display_name: String,
    auth_state: String,
    status: Option<String>,
    available: bool,
    registered: bool,
    base_url: Option<String>,
    entitlement: OfficialEntitlement,
    model_ids: Vec<String>,
    source: String,
) -> InventoryProjection {
    let models = model_ids
        .into_iter()
        .map(|model_id| {
            model_capability_summary(provider_id, account_id.as_str(), model_id, source.as_str())
        })
        .collect();
    InventoryProjection {
        account: ProviderAccountSummary {
            provider_id: provider_id.to_string(),
            account_id,
            display_name,
            auth_state,
            status,
            available,
            registered,
            base_url,
            plan_type: entitlement.plan_type,
            subscription_active_start: entitlement.subscription_active_start,
            subscription_active_until: entitlement.subscription_active_until,
            claim_source: entitlement.claim_source,
        },
        models,
    }
}

fn model_capability_summary(
    provider_id: &str,
    account_id: &str,
    model_id: String,
    source: &str,
) -> ModelCapabilitySummary {
    let normalized = model_id.to_ascii_lowercase();
    ModelCapabilitySummary {
        provider_id: provider_id.to_string(),
        account_id: account_id.to_string(),
        supports_tools: !normalized.contains("legacy"),
        supports_streaming: true,
        supports_reasoning: normalized.contains("gpt-5")
            || normalized.contains("o1")
            || normalized.contains("o3")
            || normalized.contains("claude-3-7")
            || normalized.contains("reason"),
        model_id,
        source: source.to_string(),
    }
}
