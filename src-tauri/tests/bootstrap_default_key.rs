use codexlag_lib::bootstrap::bootstrap_state_for_test;

#[tokio::test]
async fn bootstrap_creates_default_policy_and_default_key() {
    let state = bootstrap_state_for_test().await.expect("bootstrap");

    let policy = state
        .db
        .get_policy_by_name("default")
        .expect("default policy");
    let key = state
        .db
        .get_platform_key_by_name("default")
        .expect("default key");

    assert_eq!(policy.name, "default");
    assert_eq!(key.name, "default");
    assert_eq!(key.allowed_mode.as_str(), "hybrid");
}
