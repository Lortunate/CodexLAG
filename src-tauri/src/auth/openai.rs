use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use openidconnect::core::{
    CoreAuthenticationFlow, CoreJwsSigningAlgorithm, CoreProviderMetadata, CoreResponseType,
    CoreSubjectIdentifierType,
};
use openidconnect::{
    AuthUrl, ClientId, CsrfToken, EmptyAdditionalProviderMetadata, IssuerUrl,
    JsonWebKeySetUrl, Nonce, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, ResponseTypes,
    Scope, TokenUrl,
};
use serde::Serialize;

use crate::{
    error::{CodexLagError, Result},
    models::ProviderSessionSummary,
    state::AppState,
};

use super::session_store::{ProviderSessionStore, StoredProviderSession};

const OPENAI_PROVIDER_ID: &str = "openai_official";
const OPENAI_ISSUER_URL: &str = "https://auth.openai.com";
const OPENAI_AUTHORIZATION_ENDPOINT: &str = "https://auth.openai.com/oauth/authorize";
const OPENAI_TOKEN_ENDPOINT: &str = "https://auth.openai.com/oauth/token";
const OPENAI_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const OPENAI_CALLBACK_PATH: &str = "/auth/openai/callback";
const OPENAI_DEFAULT_SCOPES: &[&str] = &["openid", "email", "profile", "offline_access"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAiBrowserLoginRequest {
    pub account_id: String,
    pub display_name: String,
    pub client_id: String,
    pub issuer_url: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PendingOpenAiLoopbackAuthSession {
    pub summary: ProviderSessionSummary,
    pub authorization_url: String,
    pub callback_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAiSessionRefresh {
    pub session_secret: String,
    pub token_secret: String,
    pub expires_at_ms: Option<i64>,
    pub refreshed_at_ms: i64,
}

pub trait OpenAiSessionRefresher {
    fn refresh(&self, session: &StoredProviderSession) -> Result<OpenAiSessionRefresh>;
}

struct PendingAuthSession {
    #[allow(dead_code)]
    listener: TcpListener,
    #[allow(dead_code)]
    csrf_state: String,
    #[allow(dead_code)]
    nonce: String,
    #[allow(dead_code)]
    pkce_verifier: PkceCodeVerifier,
    #[allow(dead_code)]
    callback_url: String,
}

pub struct OpenAiAuthRuntime {
    app_state: Arc<RwLock<AppState>>,
    pending_sessions: HashMap<String, PendingAuthSession>,
}

impl OpenAiAuthRuntime {
    pub fn new(app_state: AppState) -> Self {
        Self::from_shared_app_state(Arc::new(RwLock::new(app_state)))
    }

    pub fn from_shared_app_state(app_state: Arc<RwLock<AppState>>) -> Self {
        Self {
            app_state,
            pending_sessions: HashMap::new(),
        }
    }

    pub fn app_state(&self) -> RwLockReadGuard<'_, AppState> {
        self.app_state
            .read()
            .expect("openai auth runtime app state lock poisoned")
    }

    pub fn app_state_mut(&self) -> RwLockWriteGuard<'_, AppState> {
        self.app_state
            .write()
            .expect("openai auth runtime app state lock poisoned")
    }

    pub fn store_session(
        &mut self,
        session: ProviderSessionSummary,
        session_secret: String,
        token_secret: String,
    ) -> Result<()> {
        let mut app_state = self.app_state_mut();
        ProviderSessionStore::save(
            &mut app_state,
            session,
            session_secret,
            token_secret,
        )
    }

    pub fn session(&self, account_id: &str) -> Result<Option<StoredProviderSession>> {
        let app_state = self.app_state();
        ProviderSessionStore::load(&app_state, OPENAI_PROVIDER_ID, account_id)
    }

    pub fn list_sessions(&self) -> Vec<ProviderSessionSummary> {
        let app_state = self.app_state();
        ProviderSessionStore::list(&app_state, OPENAI_PROVIDER_ID)
    }

    pub fn start_default_browser_login(
        &mut self,
        account_id: String,
        display_name: String,
    ) -> Result<PendingOpenAiLoopbackAuthSession> {
        self.start_browser_login(OpenAiBrowserLoginRequest {
            account_id,
            display_name,
            client_id: OPENAI_CLIENT_ID.into(),
            issuer_url: OPENAI_ISSUER_URL.into(),
            authorization_endpoint: OPENAI_AUTHORIZATION_ENDPOINT.into(),
            token_endpoint: OPENAI_TOKEN_ENDPOINT.into(),
            scopes: OPENAI_DEFAULT_SCOPES
                .iter()
                .map(|scope| (*scope).to_string())
                .collect(),
        })
    }

    pub fn start_browser_login(
        &mut self,
        request: OpenAiBrowserLoginRequest,
    ) -> Result<PendingOpenAiLoopbackAuthSession> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| {
            CodexLagError::new(format!("failed to bind openai loopback listener: {error}"))
        })?;
        let address = listener.local_addr().map_err(|error| {
            CodexLagError::new(format!(
                "failed to read openai loopback listener address: {error}"
            ))
        })?;
        let callback_url = format!("http://{address}{OPENAI_CALLBACK_PATH}");

        let provider_metadata = CoreProviderMetadata::new(
            IssuerUrl::new(request.issuer_url.clone()).map_err(oidc_error)?,
            AuthUrl::new(request.authorization_endpoint.clone()).map_err(oidc_error)?,
            JsonWebKeySetUrl::new(format!("{}/.well-known/jwks.json", request.issuer_url))
                .map_err(oidc_error)?,
            vec![ResponseTypes::new(vec![CoreResponseType::Code])],
            vec![CoreSubjectIdentifierType::Public],
            vec![CoreJwsSigningAlgorithm::RsaSsaPssSha256],
            EmptyAdditionalProviderMetadata {},
        )
        .set_token_endpoint(
            Some(TokenUrl::new(request.token_endpoint.clone()).map_err(oidc_error)?),
        );

        let client = openidconnect::core::CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(request.client_id.clone()),
            None,
        )
        .set_redirect_uri(RedirectUrl::new(callback_url.clone()).map_err(oidc_error)?);

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let mut auth_request = client.authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        );
        for scope in &request.scopes {
            auth_request = auth_request.add_scope(Scope::new(scope.clone()));
        }
        let (authorization_url, csrf_state, nonce) =
            auth_request.set_pkce_challenge(pkce_challenge).url();

        let summary = ProviderSessionSummary {
            provider_id: OPENAI_PROVIDER_ID.into(),
            account_id: request.account_id.clone(),
            display_name: request.display_name,
            auth_state: "pending".into(),
            refreshable: true,
            expires_at_ms: None,
            last_refresh_at_ms: None,
            last_refresh_error: None,
        };

        self.pending_sessions.insert(
            request.account_id,
            PendingAuthSession {
                listener,
                csrf_state: csrf_state.secret().to_string(),
                nonce: nonce.secret().to_string(),
                pkce_verifier,
                callback_url: callback_url.clone(),
            },
        );

        Ok(PendingOpenAiLoopbackAuthSession {
            summary,
            authorization_url: authorization_url.to_string(),
            callback_url,
        })
    }

    pub fn refresh_session_if_needed<R: OpenAiSessionRefresher>(
        &mut self,
        account_id: &str,
        now_ms: i64,
        refresher: &R,
    ) -> Result<Option<StoredProviderSession>> {
        let Some(stored) = self.session(account_id)? else {
            return Ok(None);
        };
        if !stored.summary.refreshable {
            return Ok(None);
        }
        let Some(expires_at_ms) = stored.summary.expires_at_ms else {
            return Ok(None);
        };
        if expires_at_ms > now_ms {
            return Ok(None);
        }

        let refreshed = refresher.refresh(&stored)?;
        let mut summary = stored.summary.clone();
        summary.auth_state = "active".into();
        summary.refreshable = true;
        summary.expires_at_ms = refreshed.expires_at_ms;
        summary.last_refresh_at_ms = Some(refreshed.refreshed_at_ms);
        summary.last_refresh_error = None;

        self.store_session(summary, refreshed.session_secret, refreshed.token_secret)?;
        self.session(account_id)
    }

    pub fn logout_session(&mut self, account_id: &str) -> Result<bool> {
        self.pending_sessions.remove(account_id);
        self.app_state_mut()
            .repositories_mut()
            .delete_provider_session(OPENAI_PROVIDER_ID, account_id)
    }
}

fn oidc_error(error: impl std::fmt::Display) -> CodexLagError {
    CodexLagError::new(format!("failed to configure openai auth client: {error}"))
}
