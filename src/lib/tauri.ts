import { invoke } from "@tauri-apps/api/core";
import type {
  AccountSummary,
  DefaultKeySummary,
  LogSummary,
  PolicySummary,
  RelaySummary,
} from "./types";

export function listAccounts() {
  return invoke<AccountSummary[]>("list_accounts");
}

export function listRelays() {
  return invoke<RelaySummary[]>("list_relays");
}

export function getDefaultKeySummary() {
  return invoke<DefaultKeySummary>("get_default_key_summary");
}

export function listPolicies() {
  return invoke<PolicySummary[]>("list_policies");
}

export function getLogSummary() {
  return invoke<LogSummary>("get_log_summary");
}
