use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeatureCapability {
    pub model_id: String,
    pub max_context_window: Option<u32>,
    pub supports_context_compression: Option<bool>,
    pub supports_compact_path: Option<bool>,
}

pub fn merge_cli_proxyapi_capabilities(
    model_id: &str,
    max_context_window: Option<u32>,
    supports_context_compression: Option<bool>,
    supports_compact_path: Option<bool>,
) -> FeatureCapability {
    FeatureCapability {
        model_id: model_id.to_string(),
        max_context_window,
        supports_context_compression,
        supports_compact_path,
    }
}
