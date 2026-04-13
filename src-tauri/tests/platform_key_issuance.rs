use codexlag_lib::bootstrap::bootstrap_runtime_for_test;
use codexlag_lib::commands::keys::{create_platform_key_from_runtime, CreatePlatformKeyInput};
use codexlag_lib::secret_store::SecretKey;

#[tokio::test]
async fn create_platform_key_issues_a_real_secret_and_stores_it() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let created = create_platform_key_from_runtime(
        &runtime,
        CreatePlatformKeyInput {
            key_id: "key-secondary".into(),
            name: "secondary".into(),
            policy_id: "policy-default".into(),
            allowed_mode: "hybrid".into(),
        },
    )
    .expect("create platform key");

    assert!(created.secret.starts_with("ck_local_"));
    let stored = runtime
        .app_state()
        .secret(&SecretKey::platform_key("key-secondary"))
        .expect("stored secret");
    assert_eq!(stored, created.secret);
}
