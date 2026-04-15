export type DefaultKeyMode = "account_only" | "relay_only" | "hybrid";

export type ErrorCategory =
  | "CredentialError"
  | "QuotaError"
  | "RoutingError"
  | "UpstreamError"
  | "ConfigError";

export interface AppErrorPayload {
  code: string;
  category: ErrorCategory;
  message: string;
  internal_context?: string | null;
}

export interface RawDefaultKeySummary {
  name: string;
  allowed_mode: string;
  unavailable_reason: string | null;
}

export interface DefaultKeySummary {
  name: string;
  allowedMode: DefaultKeyMode | null;
  rawAllowedMode: string;
  unavailableReason: string | null;
}

export interface AccountSummary {
  account_id: string;
  name: string;
  provider: string;
}

export interface ProviderSessionSummary {
  provider_id: string;
  account_id: string;
  display_name: string;
  auth_state: string;
  auth_profile?: string;
  expires_at_ms: number | null;
  last_refresh_at_ms: number | null;
  last_refresh_error: string | null;
  last_error_message?: string | null;
}

export interface ProviderAccountHealth {
  provider_id: string;
  account_id: string;
  auth_state: string;
  auth_profile: string;
  last_error_message: string | null;
}

export interface ProviderAccountSummary {
  provider_id: string;
  account_id: string;
  display_name: string;
  auth_state: string;
  available: boolean;
  registered: boolean;
  base_url: string | null;
}

export interface ModelCapabilitySummary {
  provider_id: string;
  account_id: string;
  model_id: string;
  supports_tools: boolean;
  supports_streaming: boolean;
  supports_reasoning: boolean;
  source: string;
}

export interface ProviderInventorySummary {
  accounts: ProviderAccountSummary[];
  models: ModelCapabilitySummary[];
}

export interface PendingOpenAiBrowserLogin {
  summary: ProviderSessionSummary;
  authorization_url: string;
  callback_url: string;
}

export interface OfficialAccountImportInput {
  account_id: string;
  name: string;
  provider: string;
  session_credential_ref: string;
  token_credential_ref: string;
  account_identity: string | null;
  auth_mode: string | null;
}

export interface AccountBalanceQueryable {
  kind: "queryable";
  total: string;
  used: string;
}

export interface AccountBalanceNonQueryable {
  kind: "non_queryable";
  reason: string;
}

export type AccountBalanceAvailability = AccountBalanceQueryable | AccountBalanceNonQueryable;

export interface AccountBalanceSnapshot {
  account_id: string;
  provider: string;
  refreshed_at: string;
  balance: AccountBalanceAvailability;
}

export interface AccountCapabilityDetail {
  account_id: string;
  provider: string;
  refresh_capability: boolean | null;
  balance_capability: string;
}

export interface RelaySummary {
  relay_id: string;
  name: string;
  endpoint: string;
}

export interface RelayUpsertInput {
  relay_id: string;
  name: string;
  endpoint: string;
  adapter?: "newapi" | "none" | "nobalance" | null;
}

export interface RelayConnectionTestResult {
  relay_id: string;
  endpoint: string;
  status: string;
  latency_ms: number;
}

export interface NormalizedBalance {
  total: string;
  used: string;
}

export interface RelayBalanceQueryable {
  kind: "queryable";
  adapter: string;
  balance: NormalizedBalance;
}

export interface RelayBalanceUnsupported {
  kind: "unsupported";
  reason: string;
}

export type RelayBalanceAvailability = RelayBalanceQueryable | RelayBalanceUnsupported;

export interface RelayBalanceSnapshot {
  relay_id: string;
  endpoint: string;
  balance: RelayBalanceAvailability;
}

export interface RelayCapabilityQueryable {
  kind: "queryable";
  adapter: string;
}

export interface RelayCapabilityUnsupported {
  kind: "unsupported";
}

export type RelayBalanceCapability = RelayCapabilityQueryable | RelayCapabilityUnsupported;

export interface RelayCapabilityDetail {
  relay_id: string;
  endpoint: string;
  balance_capability: RelayBalanceCapability;
}

export interface PlatformKeyInventoryEntry {
  id: string;
  name: string;
  policy_id: string;
  allowed_mode: DefaultKeyMode;
  enabled: boolean;
}

export interface CreatedPlatformKey {
  id: string;
  name: string;
  policy_id: string;
  allowed_mode: DefaultKeyMode;
  enabled: boolean;
  secret: string;
}

export interface CreatePlatformKeyInput {
  key_id: string;
  name: string;
  policy_id: string;
  allowed_mode: DefaultKeyMode;
}

export type UsageProvenance = "actual" | "estimated" | "unknown";

export interface UsageCost {
  amount: string | null;
  provenance: UsageProvenance;
  is_estimated: boolean;
}

export interface UsageRequestDetail {
  request_id: string;
  endpoint_id: string;
  model: string | null;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_write_tokens: number;
  reasoning_tokens: number;
  total_tokens: number;
  cost: UsageCost;
  pricing_profile_id: string | null;
  declared_capability_requirements: string | null;
  effective_capability_result: string | null;
  final_upstream_status: number | null;
  final_upstream_error_code: string | null;
  final_upstream_error_reason: string | null;
}

export interface UsageLedgerQuery {
  endpoint_id?: string;
  request_id_prefix?: string;
  limit?: number;
}

export interface UsageLedger {
  entries: UsageRequestDetail[];
  total_tokens: number;
  total_cost: UsageCost;
}

export interface PolicySummary {
  policy_id: string;
  name: string;
  status: string;
  selection_order: string[];
  cross_pool_fallback: boolean;
  retry_budget: number;
  timeout_open_after: number;
  server_error_open_after: number;
  cooldown_ms: number;
  half_open_after_ms: number;
  success_close_after: number;
}

export interface PolicyUpdateInput {
  policy_id: string;
  name: string;
  selection_order: string[];
  cross_pool_fallback: boolean;
  retry_budget: number;
  timeout_open_after: number;
  server_error_open_after: number;
  cooldown_ms: number;
  half_open_after_ms: number;
  success_close_after: number;
}

export interface PolicyPreviewSummary {
  eligible_candidates: string[];
  rejected_candidates: string[];
}

export interface LogSummary {
  last_event: string;
  level: string;
}

export interface RuntimeLogFileMetadata {
  name: string;
  path: string;
  size: number;
  mtime: number;
}

export interface RuntimeLogMetadata {
  log_dir: string;
  files: RuntimeLogFileMetadata[];
}

export interface DiagnosticsDetail {
  label: string;
  value: string;
}

export interface DiagnosticsRow {
  key: string;
  label: string;
  status: string;
  value: string;
  details: DiagnosticsDetail[];
}

export interface DiagnosticsSection {
  id: string;
  title: string;
  status: string;
  summary: string;
  rows: DiagnosticsRow[];
}

export interface ProviderDiagnosticsSummary {
  sections: DiagnosticsSection[];
}
