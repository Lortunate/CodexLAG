use codexlag_lib::providers::capabilities::{
    merge_cli_proxyapi_capabilities, FeatureCapability, FeatureCapabilityPatch,
};
use codexlag_lib::providers::official::{
    OfficialAuthMode, OfficialBalanceCapability, OfficialSession,
};
use serde_json::{from_str, to_string};

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

#[test]
fn official_auth_mode_serializes_known_values_to_stable_strings() {
    assert_eq!(
        to_string(&OfficialAuthMode::DeviceCode).expect("serialize device code auth mode"),
        "\"device_code\""
    );
    assert_eq!(
        to_string(&OfficialAuthMode::ApiKey).expect("serialize api key auth mode"),
        "\"api_key\""
    );
}

#[test]
fn official_auth_mode_deserializes_known_stable_strings() {
    assert_eq!(
        from_str::<OfficialAuthMode>("\"device_code\"").expect("deserialize device code auth mode"),
        OfficialAuthMode::DeviceCode
    );
    assert_eq!(
        from_str::<OfficialAuthMode>("\"api_key\"").expect("deserialize api key auth mode"),
        OfficialAuthMode::ApiKey
    );
}

#[test]
fn official_auth_mode_deserializes_unknown_plain_strings() {
    assert_eq!(
        from_str::<OfficialAuthMode>("\"sso\"").expect("deserialize unknown auth mode"),
        OfficialAuthMode::Unknown("sso".to_string())
    );
}

#[test]
fn official_auth_mode_round_trips_unknown_plain_strings() {
    let serialized =
        to_string(&OfficialAuthMode::Unknown("sso".to_string())).expect("serialize unknown mode");

    assert_eq!(serialized, "\"sso\"");
    assert_eq!(
        from_str::<OfficialAuthMode>(&serialized).expect("deserialize serialized unknown mode"),
        OfficialAuthMode::Unknown("sso".to_string())
    );
}

#[test]
fn official_sessions_report_balance_capability_as_non_queryable() {
    let session = OfficialSession {
        session_id: "session-balance".to_string(),
        account_identity: Some("user@example.com".to_string()),
        auth_mode: Some(OfficialAuthMode::ApiKey),
        refresh_capability: Some(true),
    };

    assert_eq!(
        session.balance_capability(),
        OfficialBalanceCapability::NonQueryable
    );
}
