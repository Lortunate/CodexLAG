use crate::error::{CodexLagError, Result};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[cfg(target_os = "windows")]
use keyring::{Entry, Error as KeyringError};

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
        Self::platform_key("key-default")
    }

    pub fn platform_key(id: impl Into<String>) -> Self {
        Self(Cow::Owned(format!("platform-key/{}", id.into())))
    }
}

pub struct SecretStore {
    adapter: Box<dyn SecretStoreAdapter>,
}

impl SecretStore {
    pub fn production() -> Result<Self> {
        #[cfg(target_os = "windows")]
        {
            return Ok(Self {
                adapter: Box::new(KeyringSecretStoreAdapter::new("codexlag")),
            });
        }

        #[cfg(not(target_os = "windows"))]
        {
            Err(CodexLagError::new(
                "platform credential storage is not configured for this OS",
            ))
        }
    }

    pub fn in_memory(namespace: impl Into<String>) -> Self {
        Self {
            adapter: Box::new(InMemorySecretStoreAdapter::new(namespace.into())),
        }
    }

    pub fn set(&self, key: &SecretKey, value: String) -> Result<()> {
        if value.is_empty() {
            return Err(CodexLagError::new(format!(
                "secret value for '{}' cannot be empty",
                key.as_str()
            )));
        }

        self.adapter.set(key, &value)
    }

    pub fn get_optional(&self, key: &SecretKey) -> Result<Option<String>> {
        let value = self.adapter.get(key)?;
        if matches!(value.as_deref(), Some("")) {
            return Err(CodexLagError::new(format!(
                "secret value for '{}' cannot be empty",
                key.as_str()
            )));
        }

        Ok(value)
    }

    pub fn get(&self, key: &SecretKey) -> Result<String> {
        self.get_optional(key)?
            .ok_or_else(|| CodexLagError::new(format!("secret '{}' not found", key.as_str())))
    }
}

impl Default for SecretStore {
    fn default() -> Self {
        Self::in_memory("test-default")
    }
}

trait SecretStoreAdapter: Send + Sync {
    fn set(&self, key: &SecretKey, value: &str) -> Result<()>;
    fn get(&self, key: &SecretKey) -> Result<Option<String>>;
}

struct InMemorySecretStoreAdapter {
    namespace: String,
}

impl InMemorySecretStoreAdapter {
    fn new(namespace: String) -> Self {
        Self { namespace }
    }

    fn shared_secrets() -> &'static Mutex<HashMap<String, String>> {
        static SECRETS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
        SECRETS.get_or_init(|| Mutex::new(HashMap::new()))
    }

    fn namespaced_key(&self, key: &SecretKey) -> String {
        format!("{}/{}", self.namespace, key.as_str())
    }
}

impl SecretStoreAdapter for InMemorySecretStoreAdapter {
    fn set(&self, key: &SecretKey, value: &str) -> Result<()> {
        let mut secrets = Self::shared_secrets()
            .lock()
            .map_err(|_| CodexLagError::new("secret store lock poisoned"))?;
        secrets.insert(self.namespaced_key(key), value.to_owned());
        Ok(())
    }

    fn get(&self, key: &SecretKey) -> Result<Option<String>> {
        let secrets = Self::shared_secrets()
            .lock()
            .map_err(|_| CodexLagError::new("secret store lock poisoned"))?;
        Ok(secrets.get(&self.namespaced_key(key)).cloned())
    }
}

#[cfg(target_os = "windows")]
struct KeyringSecretStoreAdapter {
    service_name: String,
}

#[cfg(target_os = "windows")]
impl KeyringSecretStoreAdapter {
    fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
        }
    }

    fn entry(&self, key: &SecretKey) -> Result<Entry> {
        Entry::new(&self.service_name, key.as_str()).map_err(|error| {
            CodexLagError::new(format!(
                "failed to create keyring entry for '{}': {error}",
                key.as_str()
            ))
        })
    }
}

#[cfg(target_os = "windows")]
impl SecretStoreAdapter for KeyringSecretStoreAdapter {
    fn set(&self, key: &SecretKey, value: &str) -> Result<()> {
        self.entry(key)?
            .set_password(value)
            .map_err(|error| CodexLagError::new(format!("failed to store secret: {error}")))
    }

    fn get(&self, key: &SecretKey) -> Result<Option<String>> {
        match self.entry(key)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(error) => Err(CodexLagError::new(format!(
                "failed to read secret '{}': {error}",
                key.as_str()
            ))),
        }
    }
}
