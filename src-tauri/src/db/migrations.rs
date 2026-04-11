use rusqlite::Connection;

use crate::error::{CodexLagError, Result};

pub fn ensure_schema_up_to_date(connection: &Connection) -> Result<()> {
    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS routing_policies (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE
            );

            CREATE TABLE IF NOT EXISTS platform_keys (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                allowed_mode TEXT NOT NULL,
                policy_id TEXT NOT NULL,
                enabled INTEGER NOT NULL
            );
            ",
        )
        .map_err(|error| CodexLagError::new(format!("failed to apply SQLite migrations: {error}")))?;

    Ok(())
}
