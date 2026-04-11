use std::sync::Arc;

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
};

use crate::{
    models::{PlatformKey, RoutingPolicy},
    state::AppState,
};

#[derive(Clone)]
pub struct GatewayState {
    app_state: Arc<AppState>,
}

impl GatewayState {
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self {
            app_state,
        }
    }

    pub fn app_state(&self) -> &AppState {
        self.app_state.as_ref()
    }

    pub fn policy_for_platform_key(&self, platform_key: &PlatformKey) -> Option<RoutingPolicy> {
        self.app_state
            .get_policy_by_id(&platform_key.policy_id)
            .cloned()
    }

    fn authenticate_platform_key(&self, provided_secret: &str) -> Option<PlatformKey> {
        self.app_state.authenticate_platform_key(provided_secret)
    }
}

pub struct AuthenticatedPlatformKey {
    platform_key: PlatformKey,
}

impl AuthenticatedPlatformKey {
    pub fn platform_key(&self) -> &PlatformKey {
        &self.platform_key
    }
}

impl FromRequestParts<GatewayState> for AuthenticatedPlatformKey {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &GatewayState,
    ) -> std::result::Result<Self, Self::Rejection> {
        let authorization = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let bearer_token = parse_bearer_token(authorization).ok_or(StatusCode::UNAUTHORIZED)?;
        let platform_key = state
            .authenticate_platform_key(bearer_token)
            .ok_or(StatusCode::UNAUTHORIZED)?;

        Ok(Self { platform_key })
    }
}

fn parse_bearer_token(authorization: &str) -> Option<&str> {
    let mut parts = authorization.split_whitespace();
    let scheme = parts.next()?;
    let token = parts.next()?;

    if !scheme.eq_ignore_ascii_case("bearer") || parts.next().is_some() {
        return None;
    }

    Some(token)
}
