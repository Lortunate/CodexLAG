use codexlag_lib::bootstrap::bootstrap_runtime_for_test;

#[tokio::test]
async fn runtime_starts_and_restarts_a_real_loopback_gateway_host() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let status = runtime.gateway_host_status();
    assert!(
        status.is_running,
        "gateway host should be running after bootstrap"
    );
    assert_eq!(status.listen_addr.ip().to_string(), "127.0.0.1");
    assert_eq!(status.listen_addr.port(), 8787);

    runtime.restart_gateway().expect("restart gateway");

    let restarted = runtime.gateway_host_status();
    assert!(restarted.is_running);
    assert_eq!(restarted.listen_addr.ip().to_string(), "127.0.0.1");
    assert_eq!(restarted.listen_addr.port(), 8787);
}
