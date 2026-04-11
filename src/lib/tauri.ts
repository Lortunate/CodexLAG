import { invoke } from "@tauri-apps/api/core";
import type {
  AccountSummary,
  DefaultKeySummary,
  LogSummary,
  PolicySummary,
  RawDefaultKeySummary,
  RelaySummary,
  DefaultKeyMode,
} from "./types";

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

export function listAccounts() {
  return invoke<AccountSummary[]>("list_accounts");
}

export function listRelays() {
  return invoke<RelaySummary[]>("list_relays");
}

export function getDefaultKeySummary() {
  return invoke<RawDefaultKeySummary>("get_default_key_summary").then(
    (summary): DefaultKeySummary => ({
      name: summary.name,
      allowedMode: parseDefaultKeyMode(summary.allowed_mode),
      rawAllowedMode: summary.allowed_mode,
    }),
  );
}

export function listPolicies() {
  return invoke<PolicySummary[]>("list_policies");
}

export function getLogSummary() {
  return invoke<LogSummary>("get_log_summary");
}
