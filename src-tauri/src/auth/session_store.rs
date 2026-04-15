use crate::{
    error::CodexLagError, error::Result, models::ProviderSessionSummary, secret_store::SecretKey,
    state::AppState,
};

const OPENAI_OFFICIAL_PROVIDER_ID: &str = "openai_official";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredProviderSession {
    pub summary: ProviderSessionSummary,
    pub session_secret: String,
    pub token_secret: String,
}

impl StoredProviderSession {
    pub fn is_refreshable(&self) -> bool {
        serde_json::from_str::<serde_json::Value>(&self.token_secret)
            .ok()
            .map(|value| {
                value
                    .get("refresh_token")
                    .and_then(|refresh_token| refresh_token.as_str())
                    .map(|refresh_token| !refresh_token.is_empty())
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }
}

pub struct ProviderSessionStore;

impl ProviderSessionStore {
    pub fn list(state: &AppState, provider_id: &str) -> Vec<ProviderSessionSummary> {
        let mut sessions = state
            .iter_provider_sessions()
            .filter(|session| session.provider_id == provider_id)
            .cloned()
            .collect::<Vec<_>>();
        sessions.sort_by(|left, right| left.account_id.cmp(&right.account_id));
        sessions
    }

    pub fn save(
        state: &mut AppState,
        session: ProviderSessionSummary,
        session_secret: String,
        token_secret: String,
    ) -> Result<()> {
        validate_provider(&session, OPENAI_OFFICIAL_PROVIDER_ID)?;
        state.store_secret(
            &SecretKey::new(session_secret_key(
                session.provider_id.as_str(),
                session.account_id.as_str(),
            )),
            session_secret,
        )?;
        state.store_secret(
            &SecretKey::new(token_secret_key(
                session.provider_id.as_str(),
                session.account_id.as_str(),
            )),
            token_secret,
        )?;
        state.save_provider_session(session)
    }

    pub fn load(
        state: &AppState,
        provider_id: &str,
        account_id: &str,
    ) -> Result<Option<StoredProviderSession>> {
        let Some(summary) = state
            .iter_provider_sessions()
            .find(|session| session.provider_id == provider_id && session.account_id == account_id)
            .cloned()
        else {
            return Ok(None);
        };

        let session_secret =
            state.secret(&SecretKey::new(session_secret_key(provider_id, account_id)))?;
        let token_secret =
            state.secret(&SecretKey::new(token_secret_key(provider_id, account_id)))?;

        Ok(Some(StoredProviderSession {
            summary,
            session_secret,
            token_secret,
        }))
    }
}

fn session_secret_key(provider_id: &str, account_id: &str) -> String {
    format!("credential://auth/{provider_id}/session/{account_id}")
}

fn token_secret_key(provider_id: &str, account_id: &str) -> String {
    format!("credential://auth/{provider_id}/token/{account_id}")
}

fn validate_provider(session: &ProviderSessionSummary, expected_provider_id: &str) -> Result<()> {
    if session.provider_id == expected_provider_id {
        Ok(())
    } else {
        Err(CodexLagError::new(format!(
            "provider session store expected provider '{}' but received '{}'",
            expected_provider_id, session.provider_id
        )))
    }
}
