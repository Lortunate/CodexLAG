pub const ACCOUNT_ONLY: &str = "account_only";
pub const RELAY_ONLY: &str = "relay_only";
pub const HYBRID: &str = "hybrid";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingMode {
    AccountOnly,
    RelayOnly,
    Hybrid,
}

impl RoutingMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            ACCOUNT_ONLY => Some(Self::AccountOnly),
            RELAY_ONLY => Some(Self::RelayOnly),
            HYBRID => Some(Self::Hybrid),
            _ => None,
        }
    }
}
