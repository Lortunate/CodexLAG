use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};

use crate::db::migrations::ensure_schema_up_to_date;
use crate::error::{CodexLagError, Result};
use crate::models::{PlatformKey, RoutingPolicy};

pub struct Repositories {
    database_path: PathBuf,
    policies: HashMap<String, RoutingPolicy>,
    keys: HashMap<String, PlatformKey>,
}

impl Repositories {
    pub fn open(database_path: impl AsRef<Path>) -> Result<Self> {
        let database_path = database_path.as_ref().to_path_buf();

        if let Some(parent) = database_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                CodexLagError::new(format!(
                    "failed to create database directory '{}': {error}",
                    parent.display()
                ))
            })?;
        }

        let connection = Self::open_sqlite(&database_path)?;
        ensure_schema_up_to_date(&connection)?;

        let policies = Self::load_policies(&connection)?;
        let keys = Self::load_platform_keys(&connection)?;

        Ok(Self {
            database_path,
            policies,
            keys,
        })
    }

    pub fn insert_policy(&mut self, policy: RoutingPolicy) -> Result<()> {
        let name = policy.name.clone();

        if self.policies.contains_key(&name) {
            return Err(CodexLagError::new(format!(
                "policy '{}' already exists",
                name
            )));
        }

        let connection = self.open_connection()?;

        connection
            .execute(
                "INSERT INTO routing_policies (id, name) VALUES (?1, ?2)",
                params![&policy.id, &policy.name],
            )
            .map_err(|error| {
                CodexLagError::new(format!("failed to persist policy '{}': {error}", name))
            })?;

        self.policies.insert(name, policy);
        Ok(())
    }

    pub fn insert_platform_key(&mut self, key: PlatformKey) -> Result<()> {
        let name = key.name.clone();

        if self.keys.contains_key(&name) {
            return Err(CodexLagError::new(format!(
                "platform key '{}' already exists",
                name
            )));
        }

        let connection = self.open_connection()?;

        connection
            .execute(
                "INSERT INTO platform_keys (id, name, allowed_mode, policy_id, enabled) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    &key.id,
                    &key.name,
                    &key.allowed_mode,
                    &key.policy_id,
                    key.enabled as i64
                ],
            )
            .map_err(|error| {
                CodexLagError::new(format!(
                    "failed to persist platform key '{}': {error}",
                    name
                ))
            })?;

        self.keys.insert(name, key);
        Ok(())
    }

    pub fn policy(&self, name: &str) -> Option<&RoutingPolicy> {
        self.policies.get(name)
    }

    pub fn platform_key(&self, name: &str) -> Option<&PlatformKey> {
        self.keys.get(name)
    }

    pub fn update_platform_key_allowed_mode(
        &mut self,
        name: &str,
        allowed_mode: &str,
    ) -> Result<()> {
        if !self.keys.contains_key(name) {
            return Err(CodexLagError::new(format!(
                "platform key '{}' not found",
                name
            )));
        }

        let connection = self.open_connection()?;

        connection
            .execute(
                "UPDATE platform_keys SET allowed_mode = ?1 WHERE name = ?2",
                params![allowed_mode, name],
            )
            .map_err(|error| {
                CodexLagError::new(format!(
                    "failed to update platform key '{}' mode: {error}",
                    name
                ))
            })?;

        let key = self
            .keys
            .get_mut(name)
            .expect("platform key existence checked before update");
        key.allowed_mode = allowed_mode.into();
        Ok(())
    }

    pub fn iter_policies(&self) -> impl Iterator<Item = &RoutingPolicy> {
        self.policies.values()
    }

    pub fn iter_platform_keys(&self) -> impl Iterator<Item = &PlatformKey> {
        self.keys.values()
    }

    fn open_connection(&self) -> Result<Connection> {
        Self::open_sqlite(&self.database_path)
    }

    fn open_sqlite(database_path: &Path) -> Result<Connection> {
        Connection::open(database_path).map_err(|error| {
            CodexLagError::new(format!(
                "failed to open SQLite database '{}': {error}",
                database_path.display()
            ))
        })
    }

    fn load_policies(connection: &Connection) -> Result<HashMap<String, RoutingPolicy>> {
        let mut statement = connection
            .prepare("SELECT id, name FROM routing_policies")
            .map_err(|error| {
                CodexLagError::new(format!("failed to prepare policy query: {error}"))
            })?;

        let rows = statement
            .query_map([], |row| {
                Ok(RoutingPolicy {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })
            .map_err(|error| CodexLagError::new(format!("failed to query policies: {error}")))?;

        let mut policies = HashMap::new();

        for row in rows {
            let policy = row.map_err(|error| {
                CodexLagError::new(format!("failed to decode policy row: {error}"))
            })?;
            policies.insert(policy.name.clone(), policy);
        }

        Ok(policies)
    }

    fn load_platform_keys(connection: &Connection) -> Result<HashMap<String, PlatformKey>> {
        let mut statement = connection
            .prepare(
                "SELECT id, name, allowed_mode, policy_id, enabled FROM platform_keys",
            )
            .map_err(|error| {
                CodexLagError::new(format!("failed to prepare platform key query: {error}"))
            })?;

        let rows = statement
            .query_map([], |row| {
                Ok(PlatformKey {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    allowed_mode: row.get(2)?,
                    policy_id: row.get(3)?,
                    enabled: row.get::<_, i64>(4)? != 0,
                })
            })
            .map_err(|error| {
                CodexLagError::new(format!("failed to query platform keys: {error}"))
            })?;

        let mut keys = HashMap::new();

        for row in rows {
            let key = row.map_err(|error| {
                CodexLagError::new(format!("failed to decode platform key row: {error}"))
            })?;
            keys.insert(key.name.clone(), key);
        }

        Ok(keys)
    }
}
