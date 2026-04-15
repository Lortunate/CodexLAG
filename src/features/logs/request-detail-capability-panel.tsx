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
      <h4>Capability resolution</h4>
      <p>Declared requirements</p>
      <pre>{formatMaybeJson(detail.declared_capability_requirements)}</pre>
      <p>Effective result</p>
      <pre>{formatMaybeJson(detail.effective_capability_result)}</pre>
      <p>Final route: {finalRoute}</p>
      <p>Rejected candidates: {rejectedCandidates.length > 0 ? rejectedCandidates.join(", ") : "none"}</p>
      <p>Fallback trigger: {routeExplanation?.fallback_trigger ?? "n/a"}</p>
      <p>Final routing reason: {routeExplanation?.final_reason ?? "n/a"}</p>
      <p>Final upstream status: {detail.final_upstream_status ?? "n/a"}</p>
      <p>Final upstream error code: {detail.final_upstream_error_code ?? "n/a"}</p>
      <p>Final upstream error reason: {detail.final_upstream_error_reason ?? "n/a"}</p>
    </section>
  );
}
