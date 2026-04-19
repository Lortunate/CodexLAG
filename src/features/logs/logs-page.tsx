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
    <section aria-labelledby="logs-heading">
      <PageHeader
        eyebrow="Diagnostics console"
        titleId="logs-heading"
        title="Usage Timeline"
        description="Monitor gateway health, inspect provider diagnostics, and trace request-level routing decisions from the same desktop-visible log console."
      />
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      <div className="operator-summary-grid">
        <article className="operator-callout">
          <h4>History rows</h4>
          <p>{history.length} recent requests loaded into the local operator timeline.</p>
        </article>
        <article className="operator-callout">
          <h4>Diagnostics sections</h4>
          <p>{diagnostics?.sections.length ?? 0} structured provider diagnostics groups currently available.</p>
        </article>
        <article className="operator-callout">
          <h4>Ledger provenance</h4>
          <p>{ledger?.total_cost.provenance ?? "unknown"} cost provenance on {ledger?.total_tokens ?? 0} total ledger tokens.</p>
        </article>
      </div>
      {summary ? (
        <section className="panel" aria-labelledby="logs-summary-heading">
          <div className="panel-heading">
            <div>
              <h3 id="logs-summary-heading">Runtime summary</h3>
              <p>Keep the latest operator-visible signal and current logging level in view.</p>
            </div>
          </div>
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
        </section>
      ) : null}
      {diagnostics ? (
        <section className="panel" aria-labelledby="provider-diagnostics-heading">
          <div className="panel-heading">
            <div>
              <h3 id="provider-diagnostics-heading">Provider diagnostics</h3>
              <p>Structured auth, provider, capability, and routing visibility summaries.</p>
            </div>
          </div>
          <DiagnosticsTable sections={diagnostics.sections} />
        </section>
      ) : null}
      <div className="operator-emphasis-grid">
        <section className="panel">
          <div className="panel-heading">
            <div>
              <h3>Usage provenance</h3>
              <p>Keep total token volume and cost provenance visible before drilling into request-level details.</p>
            </div>
          </div>
          <p>Total ledger tokens: {ledger?.total_tokens ?? 0}</p>
          <p>Total cost provenance: {ledger?.total_cost.provenance ?? "unknown"}</p>
          <p>Total cost estimated marker: {ledger?.total_cost.is_estimated ? "yes" : "no"}</p>
        </section>
        {requestDetail ? (
          <section className="panel">
            <div className="panel-heading">
              <div>
                <h3>Selected request</h3>
                <p>Current drill-down focus inside the request history timeline.</p>
              </div>
            </div>
            <p>Selected request id: {requestDetail.request_id}</p>
            <p>Endpoint: {requestDetail.endpoint_id}</p>
            <p>Model: {requestDetail.model ?? "n/a"}</p>
            <p>Pricing profile: {requestDetail.pricing_profile_id ?? "n/a"}</p>
          </section>
        ) : null}
      </div>
      <section className="panel">
        <div className="panel-heading">
          <div>
            <h3>Request history</h3>
            <p>Recent request rows stay compact so operators can jump into one request at a time.</p>
          </div>
        </div>
        <ul className="history-list">
          {history.map((entry) => (
            <li key={entry.request_id}>
              <div className="operator-stack">
                <div className="operator-list__item-header">
                  <strong className="operator-list__item-title">{entry.request_id}</strong>
                  <code>{entry.endpoint_id}</code>
                </div>
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
          <div className="panel-heading">
            <div>
              <h3>Request detail: {requestDetail.request_id}</h3>
              <p>Trace the final route, cost envelope, and capability projection for this single request.</p>
            </div>
          </div>
          <dl className="operator-inline-pairs">
            <div>
              <dt>Endpoint</dt>
              <dd>{requestDetail.endpoint_id}</dd>
            </div>
            <div>
              <dt>Model</dt>
              <dd>{requestDetail.model ?? "n/a"}</dd>
            </div>
            <div>
              <dt>Pricing profile</dt>
              <dd>{requestDetail.pricing_profile_id ?? "n/a"}</dd>
            </div>
            <div>
              <dt>Total tokens</dt>
              <dd>{requestDetail.total_tokens}</dd>
            </div>
          </dl>
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
