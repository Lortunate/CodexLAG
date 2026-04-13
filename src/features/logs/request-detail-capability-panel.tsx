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
  return (
    <section className="panel" aria-label="Request capability detail">
      <h4>Capability resolution</h4>
      <p>Declared requirements</p>
      <pre>{formatMaybeJson(detail.declared_capability_requirements)}</pre>
      <p>Effective result</p>
      <pre>{formatMaybeJson(detail.effective_capability_result)}</pre>
      <p>Final upstream status: {detail.final_upstream_status ?? "n/a"}</p>
      <p>Final upstream error code: {detail.final_upstream_error_code ?? "n/a"}</p>
      <p>Final upstream error reason: {detail.final_upstream_error_reason ?? "n/a"}</p>
    </section>
  );
}
