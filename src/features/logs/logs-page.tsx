import { useEffect, useState } from "react";
import {
  getLogSummary,
  getProviderDiagnostics,
  getUsageRequestDetail,
  listUsageRequestHistory,
  queryUsageLedger,
} from "../../lib/tauri";
import type {
  LogSummary,
  ProviderDiagnosticsSummary,
  UsageLedger,
  UsageRequestDetail,
} from "../../lib/types";
import { PageHeader } from "../../components/page-header";
import { DiagnosticsTable } from "./diagnostics-table";
import { RequestDetailCapabilityPanel } from "./request-detail-capability-panel";

export function LogsPage() {
  const [diagnostics, setDiagnostics] = useState<ProviderDiagnosticsSummary | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [history, setHistory] = useState<UsageRequestDetail[]>([]);
  const [ledger, setLedger] = useState<UsageLedger | null>(null);
  const [requestDetail, setRequestDetail] = useState<UsageRequestDetail | null>(null);
  const [summary, setSummary] = useState<LogSummary | null>(null);

  useEffect(() => {
    let isMounted = true;

    const loadLogs = async () => {
      try {
        const [nextSummary, nextDiagnostics, nextHistory, nextLedger] = await Promise.all([
          getLogSummary(),
          getProviderDiagnostics(),
          listUsageRequestHistory(20),
          queryUsageLedger(),
        ]);
        if (isMounted) {
          setSummary(nextSummary);
          setDiagnostics(nextDiagnostics);
          setHistory(nextHistory);
          setLedger(nextLedger);
          setErrorMessage(null);
        }
      } catch {
        if (isMounted) {
          setErrorMessage("Failed to load logs.");
        }
      }
    };

    loadLogs();

    return () => {
      isMounted = false;
    };
  }, []);

  async function handleViewRequestDetail(requestId: string) {
    try {
      const detail = await getUsageRequestDetail(requestId);
      setRequestDetail(detail);
      setErrorMessage(null);
    } catch {
      setRequestDetail(null);
      setErrorMessage(`Failed to load request detail for ${requestId}.`);
    }
  }

  return (
    <section className="workspace-page" aria-labelledby="logs-heading">
      <PageHeader
        eyebrow="Request explainability"
        title="Usage Timeline"
        titleId="logs-heading"
        description="Use desktop-visible logs to monitor the gateway, inspect provider diagnostics, and review recent request fallout."
        meta={`${history.length} recent request${history.length === 1 ? "" : "s"}`}
      />
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      {summary ? (
        <dl className="metric-list">
          <div>
            <dt>Level</dt>
            <dd>{summary.level}</dd>
          </div>
          <div>
            <dt>Last event</dt>
            <dd>{summary.last_event}</dd>
          </div>
        </dl>
      ) : null}
      {diagnostics ? (
        <section className="panel" aria-labelledby="provider-diagnostics-heading">
          <h3 id="provider-diagnostics-heading">Provider diagnostics</h3>
          <p>Structured auth, provider, capability, and routing visibility summaries.</p>
          <DiagnosticsTable sections={diagnostics.sections} />
        </section>
      ) : null}
      <section className="panel">
        <h3>Usage provenance</h3>
        <p>Total ledger tokens: {ledger?.total_tokens ?? 0}</p>
        <p>Total cost provenance: {ledger?.total_cost.provenance ?? "unknown"}</p>
        <p>Total cost estimated marker: {ledger?.total_cost.is_estimated ? "yes" : "no"}</p>
      </section>
      <section className="panel">
        <h3>Request history</h3>
        <ul className="history-list">
          {history.map((entry) => (
            <li key={entry.request_id}>
              <div>
                <strong>{entry.request_id}</strong>
                <p>Endpoint: {entry.endpoint_id}</p>
                <p>Model: {entry.model ?? "n/a"}</p>
                <p>Tokens: {entry.total_tokens}</p>
                <p>
                  {entry.cost.provenance}
                  {entry.cost.is_estimated ? " (estimated)" : ""}
                </p>
              </div>
              <button
                type="button"
                onClick={() => {
                  handleViewRequestDetail(entry.request_id);
                }}
              >
                View request {entry.request_id}
              </button>
            </li>
          ))}
        </ul>
      </section>
      {requestDetail ? (
        <section className="panel">
          <h3>Request detail: {requestDetail.request_id}</h3>
          <p>Endpoint: {requestDetail.endpoint_id}</p>
          <p>Model: {requestDetail.model ?? "n/a"}</p>
          <p>Pricing profile: {requestDetail.pricing_profile_id ?? "n/a"}</p>
          <p>
            Usage: in={requestDetail.input_tokens}, out={requestDetail.output_tokens}, cache-read=
            {requestDetail.cache_read_tokens}, cache-write={requestDetail.cache_write_tokens}, reasoning=
            {requestDetail.reasoning_tokens}
          </p>
          <p>Total tokens: {requestDetail.total_tokens}</p>
          <p>Cost provenance: {requestDetail.cost.provenance}</p>
          <p>Cost estimated marker: {requestDetail.cost.is_estimated ? "yes" : "no"}</p>
          <p>Cost amount: {requestDetail.cost.amount ?? "n/a"}</p>
          <RequestDetailCapabilityPanel detail={requestDetail} />
        </section>
      ) : null}
    </section>
  );
}
