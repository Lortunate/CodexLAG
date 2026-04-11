import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AccountSummary,
  DefaultKeySummary,
  LogSummary,
  PolicySummary,
  RawDefaultKeySummary,
  RelaySummary,
  DefaultKeyMode,
} from "./types";

export const DEFAULT_KEY_SUMMARY_CHANGED_EVENT = "default-key-summary-changed";

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
  };
}

export function listAccounts() {
  return invoke<AccountSummary[]>("list_accounts");
}

export function listRelays() {
  return invoke<RelaySummary[]>("list_relays");
}

export function getDefaultKeySummary() {
  return invoke<RawDefaultKeySummary>("get_default_key_summary").then(
    parseDefaultKeySummary,
  );
}

export function setDefaultKeyMode(mode: DefaultKeyMode) {
  return invoke<RawDefaultKeySummary>("set_default_key_mode", { mode }).then(
    parseDefaultKeySummary,
  );
}

export function listenForDefaultKeySummaryChanged(
  handler: (summary: DefaultKeySummary) => void,
) {
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
