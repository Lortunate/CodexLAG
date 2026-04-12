import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AccountBalanceSnapshot,
  AccountCapabilityDetail,
  AppErrorPayload,
  AccountSummary,
  DefaultKeyMode,
  DefaultKeySummary,
  ErrorCategory,
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

const ERROR_CATEGORIES = new Set<ErrorCategory>([
  "CredentialError",
  "QuotaError",
  "RoutingError",
  "UpstreamError",
  "ConfigError",
]);

export class CodexLagInvokeError extends Error {
  readonly payload: AppErrorPayload;
  readonly raw: unknown;

  constructor(payload: AppErrorPayload, raw: unknown) {
    super(payload.message);
    this.name = "CodexLagInvokeError";
    this.payload = payload;
    this.raw = raw;
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isErrorCategory(value: unknown): value is ErrorCategory {
  return typeof value === "string" && ERROR_CATEGORIES.has(value as ErrorCategory);
}

function categoryFromCode(code: string): ErrorCategory {
  if (code.startsWith("credential.")) {
    return "CredentialError";
  }
  if (code.startsWith("quota.")) {
    return "QuotaError";
  }
  if (code.startsWith("routing.")) {
    return "RoutingError";
  }
  if (code.startsWith("upstream.")) {
    return "UpstreamError";
  }
  return "ConfigError";
}

function parseStructuredErrorPayload(value: unknown): AppErrorPayload | null {
  if (!isRecord(value)) {
    return null;
  }

  const code = typeof value.code === "string" ? value.code : typeof value.error === "string" ? value.error : null;
  if (code) {
    const category = isErrorCategory(value.category) ? value.category : categoryFromCode(code);
    const message =
      typeof value.message === "string" ? value.message : `Request failed with error code '${code}'.`;
    const internalContext = typeof value.internal_context === "string" ? value.internal_context : null;

    return {
      code,
      category,
      message,
      internal_context: internalContext,
    };
  }

  if (isRecord(value.error)) {
    const nestedError = parseStructuredErrorPayload(value.error);
    if (nestedError) {
      return nestedError;
    }
  }

  if (isRecord(value.payload)) {
    const nestedPayload = parseStructuredErrorPayload(value.payload);
    if (nestedPayload) {
      return nestedPayload;
    }
  }

  return null;
}

function payloadFromUnknown(error: unknown): AppErrorPayload {
  const structured = parseStructuredErrorPayload(error);
  if (structured) {
    return structured;
  }

  if (typeof error === "string") {
    return parseStringError(error);
  }

  if (isRecord(error) && typeof error.message === "string") {
    return parseStringError(error.message);
  }

  return {
    code: "config.unknown",
    category: "ConfigError",
    message: "Unexpected runtime error.",
    internal_context: null,
  };
}

function parseLegacyErrorMessage(message: string): AppErrorPayload {
  const normalized = message.toLowerCase();
  if (normalized.includes("provider_auth_failed") || normalized.includes("credential")) {
    return {
      code: "credential.provider_auth_failed",
      category: "CredentialError",
      message,
      internal_context: null,
    };
  }
  if (normalized.includes("rate limit") || normalized.includes("429")) {
    return {
      code: "quota.provider_rate_limited",
      category: "QuotaError",
      message,
      internal_context: null,
    };
  }
  if (normalized.includes("no available endpoint") || normalized.includes("invalid mode")) {
    return {
      code: normalized.includes("invalid mode") ? "routing.invalid_mode" : "routing.no_available_endpoint",
      category: "RoutingError",
      message,
      internal_context: null,
    };
  }
  if (
    normalized.includes("timeout") ||
    normalized.includes("upstream") ||
    normalized.includes("payload parse error")
  ) {
    return {
      code: normalized.includes("payload parse error")
        ? "upstream.relay_payload_invalid"
        : "upstream.provider_http_failure",
      category: "UpstreamError",
      message,
      internal_context: null,
    };
  }
  return {
    code: "config.unknown",
    category: "ConfigError",
    message,
    internal_context: null,
  };
}

function parseStringError(value: string): AppErrorPayload {
  try {
    const parsed = JSON.parse(value) as unknown;
    const structured = parseStructuredErrorPayload(parsed);
    if (structured) {
      return structured;
    }
  } catch {
    // fall through to legacy parser
  }
  return parseLegacyErrorMessage(value);
}

function normalizeInvokeError(error: unknown): CodexLagInvokeError {
  return new CodexLagInvokeError(payloadFromUnknown(error), error);
}

async function invokeWithContract<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    if (args === undefined) {
      return await invoke<T>(command);
    }
    return await invoke<T>(command, args);
  } catch (error) {
    throw normalizeInvokeError(error);
  }
}

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
  return invokeWithContract<AccountSummary[]>("list_accounts");
}

export function refreshAccountBalance(accountId: string) {
  return invokeWithContract<AccountBalanceSnapshot>("refresh_account_balance", { account_id: accountId });
}

export function getAccountCapabilityDetail(accountId: string) {
  return invokeWithContract<AccountCapabilityDetail>("get_account_capability_detail", { account_id: accountId });
}

export function listRelays() {
  return invokeWithContract<RelaySummary[]>("list_relays");
}

export function refreshRelayBalance(relayId: string) {
  return invokeWithContract<RelayBalanceSnapshot>("refresh_relay_balance", { relay_id: relayId });
}

export function getRelayCapabilityDetail(relayId: string) {
  return invokeWithContract<
    Omit<RelayCapabilityDetail, "balance_capability"> & { balance_capability: RawRelayBalanceCapability }
  >(
    "get_relay_capability_detail",
    { relay_id: relayId },
  ).then((detail) => ({
    ...detail,
    balance_capability: parseRelayCapability(detail.balance_capability),
  }));
}

export function getDefaultKeySummary() {
  return invokeWithContract<RawDefaultKeySummary>("get_default_key_summary").then(parseDefaultKeySummary);
}

export function setDefaultKeyMode(mode: DefaultKeyMode) {
  return invokeWithContract<RawDefaultKeySummary>("set_default_key_mode", { mode }).then(parseDefaultKeySummary);
}

export function listenForDefaultKeySummaryChanged(handler: (summary: DefaultKeySummary) => void) {
  return listen<RawDefaultKeySummary>(DEFAULT_KEY_SUMMARY_CHANGED_EVENT, (event) => {
    handler(parseDefaultKeySummary(event.payload));
  });
}

export function listPolicies() {
  return invokeWithContract<PolicySummary[]>("list_policies");
}

export function getLogSummary() {
  return invokeWithContract<LogSummary>("get_log_summary");
}

export function getRuntimeLogMetadata() {
  return invokeWithContract<RuntimeLogMetadata>("get_runtime_log_metadata");
}

export function exportRuntimeDiagnostics() {
  return invokeWithContract<string>("export_runtime_diagnostics");
}

export function getUsageRequestDetail(requestId: string) {
  return invokeWithContract<UsageRequestDetail | null>("get_usage_request_detail", { request_id: requestId });
}

export function listUsageRequestHistory(limit?: number) {
  return invokeWithContract<UsageRequestDetail[]>("list_usage_request_history", { limit });
}

export function queryUsageLedger(query?: UsageLedgerQuery) {
  return invokeWithContract<UsageLedger>("query_usage_ledger", { query });
}
