use std::collections::HashMap;

#[derive(Default)]
pub struct SecretStore {
    secrets: HashMap<String, String>,
}

impl SecretStore {
    pub fn set(&mut self, key: &str, value: String) {
        self.secrets.insert(key.to_string(), value);
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.secrets.get(key).cloned()
    }
}
