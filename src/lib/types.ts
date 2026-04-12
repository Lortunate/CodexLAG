export type DefaultKeyMode = "account_only" | "relay_only" | "hybrid";

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

export type UsageProvenance = "estimated" | "unknown";

export interface UsageCost {
  amount: string | null;
  provenance: UsageProvenance;
}

export interface UsageRequestDetail {
  request_id: string;
  endpoint_id: string;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_write_tokens: number;
  total_tokens: number;
  cost: UsageCost;
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
  name: string;
  status: string;
}

export interface LogSummary {
  last_event: string;
  level: string;
}

export interface RuntimeLogMetadata {
  log_dir: string;
  files: string[];
}
