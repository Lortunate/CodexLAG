export type DefaultKeyMode = "account_only" | "relay_only" | "hybrid";

export interface AccountSummary {
  name: string;
  provider: string;
}

export interface RelaySummary {
  name: string;
  endpoint: string;
}

export interface DefaultKeySummary {
  name: string;
  allowed_mode: DefaultKeyMode;
}

export interface PolicySummary {
  name: string;
  status: string;
}

export interface LogSummary {
  last_event: string;
  level: string;
}
