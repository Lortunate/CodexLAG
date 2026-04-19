import type { UsageRequestDetail } from "../../lib/types";

interface RequestDetailCapabilityPanelProps {
  detail: UsageRequestDetail;
}

function formatMaybeJson(value: string | null): string {
  if (!value) {
    return "n/a";
  }

  try {
    return JSON.stringify(JSON.parse(value), null, 2);
  } catch {
    return value;
  }
}

export function RequestDetailCapabilityPanel({ detail }: RequestDetailCapabilityPanelProps) {
  const routeExplanation = detail.route_explanation;
  const finalRoute = routeExplanation
    ? routeExplanation.selected_candidate_id ?? "not persisted"
    : "not recorded";
  const rejectedCandidates = routeExplanation?.rejected_candidates ?? [];

  return (
    <section className="panel" aria-label="Request capability detail">
      <div className="panel-heading">
        <div>
          <h4>Capability resolution</h4>
          <p>Compare the declared requirement set against the effective runtime result and final route choice.</p>
        </div>
      </div>
      <div className="operator-emphasis-grid">
        <section className="operator-preview">
          <h4>Declared requirements</h4>
          <pre className="operator-code-block">{formatMaybeJson(detail.declared_capability_requirements)}</pre>
        </section>
        <section className="operator-preview">
          <h4>Effective result</h4>
          <pre className="operator-code-block">{formatMaybeJson(detail.effective_capability_result)}</pre>
        </section>
      </div>
      <div className="operator-preview">
        <h4>Route explanation</h4>
        <p>Final route: {finalRoute}</p>
        <p>
          Rejected candidates: {rejectedCandidates.length > 0 ? rejectedCandidates.join(", ") : "none"}
        </p>
        <p>Fallback trigger: {routeExplanation?.fallback_trigger ?? "n/a"}</p>
        <p>Final routing reason: {routeExplanation?.final_reason ?? "n/a"}</p>
      </div>
      <dl className="operator-inline-pairs">
        <div>
          <dt>Final upstream status</dt>
          <dd>{detail.final_upstream_status ?? "n/a"}</dd>
        </div>
        <div>
          <dt>Final error code</dt>
          <dd>{detail.final_upstream_error_code ?? "n/a"}</dd>
        </div>
        <div>
          <dt>Final error reason</dt>
          <dd>{detail.final_upstream_error_reason ?? "n/a"}</dd>
        </div>
      </dl>
      <p>Final upstream status: {detail.final_upstream_status ?? "n/a"}</p>
      <p>Final upstream error code: {detail.final_upstream_error_code ?? "n/a"}</p>
      <p>Final upstream error reason: {detail.final_upstream_error_reason ?? "n/a"}</p>
    </section>
  );
}
