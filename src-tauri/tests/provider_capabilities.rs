use codexlag_lib::providers::capabilities::merge_cli_proxyapi_capabilities;

#[test]
fn capability_merge_includes_registered_max_tokens() {
    let capability = merge_cli_proxyapi_capabilities(
        "claude-3-5-sonnet",
        Some(8192),
        Some(false),
        Some(true),
    );

    assert_eq!(capability.max_context_window, Some(8192));
    assert_eq!(capability.supports_context_compression, Some(false));
    assert_eq!(capability.supports_compact_path, Some(true));
}
