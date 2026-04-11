use crate::error::{CodexLagError, Result};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SecretKey(&'static str);

impl SecretKey {
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    pub fn as_str(&self) -> &str {
        self.0
    }

    pub const PLATFORM_KEY_DEFAULT: Self = SecretKey::new("platform-key/default");
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
