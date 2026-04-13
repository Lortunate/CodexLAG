use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeatureCapability {
    pub model_id: String,
    pub max_context_window: Option<u32>,
    pub supports_context_compression: Option<bool>,
    pub supports_compact_path: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeatureCapabilityPatch {
    pub max_context_window: Option<u32>,
    pub supports_context_compression: Option<bool>,
    pub supports_compact_path: Option<bool>,
}

pub fn merge_cli_proxyapi_capabilities(
    base: FeatureCapability,
    overlay: FeatureCapabilityPatch,
) -> FeatureCapability {
    FeatureCapability {
        model_id: base.model_id,
        max_context_window: overlay.max_context_window.or(base.max_context_window),
        supports_context_compression: overlay
            .supports_context_compression
            .or(base.supports_context_compression),
        supports_compact_path: overlay.supports_compact_path.or(base.supports_compact_path),
    }
}
