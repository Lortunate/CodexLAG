import { useEffect, useState } from "react";
import {
  exportRuntimeDiagnostics,
  getDefaultKeySummary,
  getLogSummary,
  getRuntimeLogMetadata,
  listAccounts,
  listRelays,
  listenForDefaultKeySummaryChanged,
  queryUsageLedger,
  refreshAccountBalance,
  refreshRelayBalance,
  setDefaultKeyMode,
} from "../../lib/tauri";
import type {
  AccountBalanceSnapshot,
  DefaultKeySummary,
  LogSummary,
  RelayBalanceSnapshot,
  RuntimeLogMetadata,
  UsageLedger,
} from "../../lib/types";
import { DefaultKeyModeToggle } from "../default-key/default-key-mode-toggle";

const initialSummary: DefaultKeySummary = {
  name: "loading",
  allowedMode: null,
  rawAllowedMode: "loading",
  unavailableReason: null,
};

export function OverviewPage() {
  const [accountBalances, setAccountBalances] = useState<AccountBalanceSnapshot[]>([]);
  const [accountRefreshFailures, setAccountRefreshFailures] = useState(0);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [isUpdatingMode, setIsUpdatingMode] = useState(false);
  const [logSummary, setLogSummary] = useState<LogSummary | null>(null);
  const [relayRefreshFailures, setRelayRefreshFailures] = useState(0);
  const [relayBalances, setRelayBalances] = useState<RelayBalanceSnapshot[]>([]);
  const [runtimeLogMetadata, setRuntimeLogMetadata] = useState<RuntimeLogMetadata | null>(null);
  const [runtimeLogDiagnosticsUnavailable, setRuntimeLogDiagnosticsUnavailable] = useState(false);
  const [runtimeDiagnosticsManifestPath, setRuntimeDiagnosticsManifestPath] = useState<string | null>(null);
  const [isExportingDiagnostics, setIsExportingDiagnostics] = useState(false);
  const [summary, setSummary] = useState<DefaultKeySummary>(initialSummary);
  const [usageLedger, setUsageLedger] = useState<UsageLedger | null>(null);

  useEffect(() => {
    let isMounted = true;
    let disposeListener: (() => void) | null = null;

    const loadOverview = async () => {
      try {
        const [nextSummary, nextLogSummary, accountList, relayList, nextLedger] = await Promise.all([
          getDefaultKeySummary(),
          getLogSummary(),
          listAccounts(),
          listRelays(),
          queryUsageLedger(),
        ]);

        const [nextAccountBalances, nextRelayBalances] = await Promise.all([
          Promise.all(
            accountList.map(async (account) => {
              try {
                return await refreshAccountBalance(account.account_id);
              } catch {
                return null;
              }
            }),
          ),
          Promise.all(
            relayList.map(async (relay) => {
              try {
                return await refreshRelayBalance(relay.relay_id);
              } catch {
                return null;
              }
            }),
          ),
        ]);

        if (!isMounted) {
          return;
        }

        setSummary(nextSummary);
        setLogSummary(nextLogSummary);
        setAccountBalances(
          nextAccountBalances.filter((snapshot): snapshot is AccountBalanceSnapshot => snapshot !== null),
        );
        setRelayBalances(
          nextRelayBalances.filter((snapshot): snapshot is RelayBalanceSnapshot => snapshot !== null),
        );
        setAccountRefreshFailures(nextAccountBalances.filter((snapshot) => snapshot === null).length);
        setRelayRefreshFailures(nextRelayBalances.filter((snapshot) => snapshot === null).length);
        setUsageLedger(nextLedger);
        setErrorMessage(null);
      } catch {
        if (isMounted) {
          setErrorMessage("Failed to load overview status.");
        }
      }
    };

    const loadRuntimeDiagnostics = async () => {
      try {
        const nextRuntimeLogMetadata = await getRuntimeLogMetadata();
        if (!isMounted) {
          return;
        }

        setRuntimeLogMetadata(nextRuntimeLogMetadata);
        setRuntimeLogDiagnosticsUnavailable(false);
      } catch {
        if (isMounted) {
          setRuntimeLogMetadata(null);
          setRuntimeLogDiagnosticsUnavailable(true);
        }
      }
    };

    loadOverview();
    loadRuntimeDiagnostics();

    listenForDefaultKeySummaryChanged((nextSummary) => {
      if (isMounted) {
        setSummary(nextSummary);
        setErrorMessage(null);
      }
    })
      .then((unlisten) => {
        if (isMounted) {
          disposeListener = unlisten;
        } else {
          unlisten();
        }
      })
      .catch(() => {
        if (isMounted) {
          setErrorMessage("Failed to subscribe to default key mode updates.");
        }
      });

    return () => {
      isMounted = false;
      disposeListener?.();
    };
  }, []);

  async function handleSelectMode(mode: "account_only" | "relay_only" | "hybrid") {
    if (isUpdatingMode || summary.allowedMode === mode) {
      return;
    }

    setIsUpdatingMode(true);
    try {
      const nextSummary = await setDefaultKeyMode(mode);
      setSummary(nextSummary);
      setErrorMessage(null);
    } catch {
      setErrorMessage("Failed to update default key mode.");
    } finally {
      setIsUpdatingMode(false);
    }
  }

  async function handleExportDiagnostics() {
    if (isExportingDiagnostics) {
      return;
    }

    setIsExportingDiagnostics(true);
    try {
      const manifestPath = await exportRuntimeDiagnostics();
      setRuntimeDiagnosticsManifestPath(manifestPath);
    } catch {
      setRuntimeDiagnosticsManifestPath("Export failed");
    } finally {
      setIsExportingDiagnostics(false);
    }
  }

  const nonQueryableAccountCount = accountBalances.filter(
    (snapshot) => snapshot.balance.kind === "non_queryable",
  ).length;
  const queryableRelayCount = relayBalances.filter(
    (snapshot) => snapshot.balance.kind === "queryable",
  ).length;

  return (
    <section aria-labelledby="overview-heading">
      <h2 id="overview-heading">Gateway Overview</h2>
      <p>CodexLAG manages local accounts, relays, keys, policy routing, and logs.</p>
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      <div className="status-card-grid">
        <article className="status-card">
          <h3>Runtime status</h3>
          <p>Level: {logSummary?.level ?? "loading"}</p>
          <p>{logSummary?.last_event ?? "Loading runtime summary..."}</p>
        </article>
        <article className="status-card">
          <h3>Balance observability</h3>
          <p>Non-queryable accounts: {nonQueryableAccountCount}</p>
          <p>Queryable relays: {queryableRelayCount}</p>
          <p>Account refresh failures: {accountRefreshFailures}</p>
          <p>Relay refresh failures: {relayRefreshFailures}</p>
        </article>
        <article className="status-card">
          <h3>Usage ledger</h3>
          <p>Total ledger tokens: {usageLedger?.total_tokens ?? 0}</p>
          <p>Usage cost provenance: {usageLedger?.total_cost.provenance ?? "unknown"}</p>
        </article>
        <article className="status-card">
          <h3>Runtime diagnostics</h3>
          <p>
            Log directory:{" "}
            {runtimeLogMetadata?.log_dir ?? (runtimeLogDiagnosticsUnavailable ? "unavailable" : "loading")}
          </p>
          <p>Tracked log files: {runtimeLogMetadata?.files.length ?? 0}</p>
          <button type="button" onClick={handleExportDiagnostics} disabled={isExportingDiagnostics}>
            Export diagnostics
          </button>
          {runtimeDiagnosticsManifestPath ? (
            <p>Diagnostics manifest: {runtimeDiagnosticsManifestPath}</p>
          ) : null}
        </article>
      </div>
      <DefaultKeyModeToggle
        activeMode={summary.allowedMode}
        disabled={isUpdatingMode}
        rawMode={summary.rawAllowedMode}
        unavailableReason={summary.unavailableReason}
        summaryName={summary.name}
        onSelectMode={handleSelectMode}
      />
    </section>
  );
}
