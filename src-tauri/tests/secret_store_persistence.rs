use codexlag_lib::{
    bootstrap::bootstrap_state_for_test_at,
    secret_store::{SecretKey, SecretStore},
};
use rand::{rngs::OsRng, RngCore};

fn unique_database_path(prefix: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("{prefix}-{}.sqlite3", random_suffix()))
}

fn random_suffix() -> String {
    let mut bytes = [0_u8; 16];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[tokio::test]
async fn secret_store_persistence_restores_platform_key_secret_across_bootstrap() {
    let database_path = unique_database_path("codexlag-secret-store-persistence");

    let first_state = bootstrap_state_for_test_at(&database_path)
        .await
        .expect("first bootstrap");
    let default_key = first_state
        .get_platform_key_by_name("default")
        .expect("default key");
    let default_key_secret = SecretKey::platform_key(default_key.id.clone());
    let expected_secret = "ck_local_custom_persisted_value".to_string();

    first_state
        .store_secret(&default_key_secret, expected_secret.clone())
        .expect("store custom default key secret");
    drop(first_state);

    let second_state = bootstrap_state_for_test_at(&database_path)
        .await
        .expect("second bootstrap");
    let restored_secret = second_state
        .secret(&default_key_secret)
        .expect("restore custom secret from secret store");

    assert_eq!(restored_secret, expected_secret);

    let _ = std::fs::remove_file(&database_path);
}

#[test]
fn secret_store_persistence_rejects_missing_and_empty_secrets() {
    let secret_store = SecretStore::in_memory(format!("test/secret-store/{}", random_suffix()));
    let missing_key = SecretKey::new("missing-secret");

    let missing = secret_store
        .get(&missing_key)
        .expect_err("missing secret should fail");
    assert!(missing.to_string().contains("not found"));

    let empty = secret_store
        .set(&missing_key, String::new())
        .expect_err("empty secret should fail");
    assert!(empty.to_string().contains("cannot be empty"));
}
