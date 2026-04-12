import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AccountBalanceSnapshot,
  AccountCapabilityDetail,
  AccountSummary,
  DefaultKeyMode,
  DefaultKeySummary,
  LogSummary,
  PolicySummary,
  RawDefaultKeySummary,
  RelayBalanceCapability,
  RelayBalanceSnapshot,
  RelayCapabilityDetail,
  RelaySummary,
  RuntimeLogMetadata,
  UsageLedger,
  UsageLedgerQuery,
  UsageRequestDetail,
} from "./types";

export const DEFAULT_KEY_SUMMARY_CHANGED_EVENT = "default-key-summary-changed";

type RawRelayBalanceCapability =
  | "unsupported"
  | {
      queryable?: {
        adapter?: string;
      };
    };

function parseDefaultKeyMode(value: string): DefaultKeyMode | null {
  switch (value) {
    case "account_only":
    case "relay_only":
    case "hybrid":
      return value;
    default:
      return null;
  }
}

function parseDefaultKeySummary(summary: RawDefaultKeySummary): DefaultKeySummary {
  return {
    name: summary.name,
    allowedMode: parseDefaultKeyMode(summary.allowed_mode),
    rawAllowedMode: summary.allowed_mode,
    unavailableReason: summary.unavailable_reason,
  };
}

function parseRelayCapability(rawCapability: RawRelayBalanceCapability): RelayBalanceCapability {
  if (rawCapability === "unsupported") {
    return { kind: "unsupported" };
  }

  if (
    rawCapability &&
    typeof rawCapability === "object" &&
    rawCapability.queryable &&
    typeof rawCapability.queryable === "object" &&
    typeof rawCapability.queryable.adapter === "string"
  ) {
    return {
      kind: "queryable",
      adapter: rawCapability.queryable.adapter,
    };
  }

  return { kind: "unsupported" };
}

export function listAccounts() {
  return invoke<AccountSummary[]>("list_accounts");
}

export function refreshAccountBalance(accountId: string) {
  return invoke<AccountBalanceSnapshot>("refresh_account_balance", { account_id: accountId });
}

export function getAccountCapabilityDetail(accountId: string) {
  return invoke<AccountCapabilityDetail>("get_account_capability_detail", { account_id: accountId });
}

export function listRelays() {
  return invoke<RelaySummary[]>("list_relays");
}

export function refreshRelayBalance(relayId: string) {
  return invoke<RelayBalanceSnapshot>("refresh_relay_balance", { relay_id: relayId });
}

export function getRelayCapabilityDetail(relayId: string) {
  return invoke<Omit<RelayCapabilityDetail, "balance_capability"> & { balance_capability: RawRelayBalanceCapability }>(
    "get_relay_capability_detail",
    { relay_id: relayId },
  ).then((detail) => ({
    ...detail,
    balance_capability: parseRelayCapability(detail.balance_capability),
  }));
}

export function getDefaultKeySummary() {
  return invoke<RawDefaultKeySummary>("get_default_key_summary").then(parseDefaultKeySummary);
}

export function setDefaultKeyMode(mode: DefaultKeyMode) {
  return invoke<RawDefaultKeySummary>("set_default_key_mode", { mode }).then(parseDefaultKeySummary);
}

export function listenForDefaultKeySummaryChanged(handler: (summary: DefaultKeySummary) => void) {
  return listen<RawDefaultKeySummary>(DEFAULT_KEY_SUMMARY_CHANGED_EVENT, (event) => {
    handler(parseDefaultKeySummary(event.payload));
  });
}

export function listPolicies() {
  return invoke<PolicySummary[]>("list_policies");
}

export function getLogSummary() {
  return invoke<LogSummary>("get_log_summary");
}

export function getRuntimeLogMetadata() {
  return invoke<RuntimeLogMetadata>("get_runtime_log_metadata");
}

export function exportRuntimeDiagnostics() {
  return invoke<string>("export_runtime_diagnostics");
}

export function getUsageRequestDetail(requestId: string) {
  return invoke<UsageRequestDetail | null>("get_usage_request_detail", { request_id: requestId });
}

export function listUsageRequestHistory(limit?: number) {
  return invoke<UsageRequestDetail[]>("list_usage_request_history", { limit });
}

export function queryUsageLedger(query?: UsageLedgerQuery) {
  return invoke<UsageLedger>("query_usage_ledger", { query });
}
