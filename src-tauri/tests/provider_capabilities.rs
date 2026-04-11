use codexlag_lib::providers::capabilities::{
    merge_cli_proxyapi_capabilities, FeatureCapability, FeatureCapabilityPatch,
};
use codexlag_lib::providers::official::{OfficialAuthMode, OfficialSession};

#[test]
fn capability_merge_overlays_registered_values() {
    let capability = merge_cli_proxyapi_capabilities(
        FeatureCapability {
            model_id: "claude-3-5-sonnet".to_string(),
            max_context_window: Some(4096),
            supports_context_compression: Some(true),
            supports_compact_path: None,
        },
        FeatureCapabilityPatch {
            max_context_window: Some(8192),
            supports_context_compression: Some(false),
            supports_compact_path: Some(true),
        },
    );

    assert_eq!(capability.model_id, "claude-3-5-sonnet");
    assert_eq!(capability.max_context_window, Some(8192));
    assert_eq!(capability.supports_context_compression, Some(false));
    assert_eq!(capability.supports_compact_path, Some(true));
}

#[test]
fn capability_merge_preserves_base_values_when_overlay_omits_them() {
    let capability = merge_cli_proxyapi_capabilities(
        FeatureCapability {
            model_id: "claude-3-5-sonnet".to_string(),
            max_context_window: Some(8192),
            supports_context_compression: Some(false),
            supports_compact_path: Some(true),
        },
        FeatureCapabilityPatch::default(),
    );

    assert_eq!(capability.model_id, "claude-3-5-sonnet");
    assert_eq!(capability.max_context_window, Some(8192));
    assert_eq!(capability.supports_context_compression, Some(false));
    assert_eq!(capability.supports_compact_path, Some(true));
}

#[test]
fn official_session_supports_unloaded_metadata_state() {
    let session = OfficialSession {
        session_id: "session-1".to_string(),
        account_identity: None,
        auth_mode: None,
        refresh_capability: None,
    };

    assert_eq!(session.account_identity, None);
    assert_eq!(session.auth_mode, None);
    assert_eq!(session.refresh_capability, None);
}

#[test]
fn official_session_can_represent_unknown_auth_mode() {
    let session = OfficialSession {
        session_id: "session-1".to_string(),
        account_identity: Some("user@example.com".to_string()),
        auth_mode: Some(OfficialAuthMode::Unknown("sso".to_string())),
        refresh_capability: Some(true),
    };

    assert_eq!(
        session.auth_mode,
        Some(OfficialAuthMode::Unknown("sso".to_string()))
    );
}
