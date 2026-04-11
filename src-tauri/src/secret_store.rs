use crate::error::{CodexLagError, Result};
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SecretKey(Cow<'static, str>);

impl SecretKey {
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }

    pub fn default_platform_key() -> Self {
        Self::platform_key("default")
    }

    pub fn platform_key(name: impl Into<String>) -> Self {
        Self(Cow::Owned(format!("platform-key/{}", name.into())))
    }
}

#[derive(Default)]
pub struct SecretStore {
    secrets: HashMap<String, String>,
}

impl SecretStore {
    pub fn set(&mut self, key: &SecretKey, value: String) -> Result<()> {
        if value.is_empty() {
            return Err(CodexLagError::new(format!(
                "secret value for '{}' cannot be empty",
                key.as_str()
            )));
        }

        self.secrets.insert(key.as_str().to_string(), value);
        Ok(())
    }

    pub fn get(&self, key: &SecretKey) -> Result<String> {
        self.secrets
            .get(key.as_str())
            .cloned()
            .ok_or_else(|| CodexLagError::new(format!("secret '{}' not found", key.as_str())))
    }
}
