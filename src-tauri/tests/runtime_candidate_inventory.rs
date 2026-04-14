use codexlag_lib::bootstrap::bootstrap_runtime_for_test;

#[tokio::test]
async fn runtime_candidates_are_built_from_persisted_accounts_and_relays() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let candidates = runtime.loopback_gateway().state().current_candidates();
    assert!(
        candidates
            .iter()
            .any(|candidate| candidate.id == "official-primary"),
        "official inventory should produce candidate official-primary"
    );
    assert!(
        candidates
            .iter()
            .any(|candidate| candidate.id == "relay-newapi"),
        "relay inventory should produce candidate relay-newapi"
    );
}
