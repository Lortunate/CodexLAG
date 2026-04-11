use codexlag_lib::{bootstrap::bootstrap_state_for_test, secret_store::SecretKey};

const DEFAULT_PLATFORM_KEY_SECRET_PREFIX: &str = "ck_local_";

#[tokio::test]
async fn bootstrap_creates_default_policy_and_default_key() {
    let state = bootstrap_state_for_test().await.expect("bootstrap");

    let policy = state.get_policy_by_name("default").expect("default policy");
    let key = state
        .get_platform_key_by_name("default")
        .expect("default key");

    assert_eq!(policy.name, "default");
    assert_eq!(key.name, "default");
    assert_eq!(key.allowed_mode.as_str(), "hybrid");
}

#[tokio::test]
async fn bootstrap_persists_default_key_secret_in_secret_store() {
    let state = bootstrap_state_for_test().await.expect("bootstrap");

    let secret = state
        .secret(&SecretKey::PLATFORM_KEY_DEFAULT)
        .expect("default key secret");

    assert!(secret.starts_with(DEFAULT_PLATFORM_KEY_SECRET_PREFIX));
    assert!(secret.len() > DEFAULT_PLATFORM_KEY_SECRET_PREFIX.len());
}
