use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
};

#[derive(Clone)]
pub struct GatewayAuthState {
    expected_secret: String,
}

impl GatewayAuthState {
    pub fn new(expected_secret: impl Into<String>) -> Self {
        Self {
            expected_secret: expected_secret.into(),
        }
    }

    fn matches_bearer_token(&self, authorization: &str) -> bool {
        authorization
            .strip_prefix("Bearer ")
            .is_some_and(|token| token == self.expected_secret)
    }
}

pub struct PlatformKeyAuth;

impl<S> FromRequestParts<S> for PlatformKeyAuth
where
    GatewayAuthState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        let auth_state = GatewayAuthState::from_ref(state);
        let authorization = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        if auth_state.matches_bearer_token(authorization) {
            Ok(Self)
        } else {
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
