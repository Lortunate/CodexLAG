use crate::{
    error::Result,
    models::ProviderSessionSummary,
    state::AppState,
};

use super::session_store::{ProviderSessionStore, StoredProviderSession};

const OPENAI_OFFICIAL_PROVIDER_ID: &str = "openai_official";

pub struct OpenAiAuthRuntime {
    app_state: AppState,
}

impl OpenAiAuthRuntime {
    pub fn new(app_state: AppState) -> Self {
        Self { app_state }
    }

    pub fn app_state(&self) -> &AppState {
        &self.app_state
    }

    pub fn app_state_mut(&mut self) -> &mut AppState {
        &mut self.app_state
    }

    pub fn store_session(
        &mut self,
        session: ProviderSessionSummary,
        session_secret: String,
        token_secret: String,
    ) -> Result<()> {
        ProviderSessionStore::save(
            self.app_state_mut(),
            session,
            session_secret,
            token_secret,
        )
    }

    pub fn session(&self, account_id: &str) -> Result<Option<StoredProviderSession>> {
        ProviderSessionStore::load(self.app_state(), OPENAI_OFFICIAL_PROVIDER_ID, account_id)
    }

    pub fn list_sessions(&self) -> Vec<ProviderSessionSummary> {
        ProviderSessionStore::list(self.app_state(), OPENAI_OFFICIAL_PROVIDER_ID)
    }
}
