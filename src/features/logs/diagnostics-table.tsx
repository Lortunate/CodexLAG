import type { DiagnosticsSection } from "../../lib/types";

interface DiagnosticsTableProps {
  sections: DiagnosticsSection[];
}

function statusTone(status: string) {
  switch (status) {
    case "error":
      return "text-[color:oklch(0.72_0.16_28)]";
    case "warn":
      return "text-[color:oklch(0.8_0.11_85)]";
    case "healthy":
      return "text-[color:oklch(0.84_0.08_160)]";
    default:
      return "text-muted-foreground";
  }
}

export function DiagnosticsTable({ sections }: DiagnosticsTableProps) {
  if (sections.length === 0) {
    return <p>No diagnostics have been captured yet.</p>;
  }

  return (
    <div className="space-y-4">
      {sections.map((section) => (
        <section key={section.id} className="operator-preview">
          <div className="panel-heading">
            <div>
              <h4>{section.title}</h4>
              <p>{section.summary}</p>
            </div>
            <span
              className={`inline-flex items-center rounded-full border border-border/70 px-2.5 py-1 text-xs font-semibold uppercase tracking-[0.12em] ${statusTone(section.status)}`}
            >
              {section.status}
            </span>
          </div>
          {section.rows.length === 0 ? (
            <p className="text-sm text-muted-foreground">No rows recorded for this section.</p>
          ) : (
            <div className="operator-table-shell">
              <table aria-label={section.title}>
                <thead>
                  <tr>
                    <th>Item</th>
                    <th>Status</th>
                    <th>Value</th>
                    <th>Details</th>
                  </tr>
                </thead>
                <tbody>
                  {section.rows.map((row) => (
                    <tr key={row.key}>
                      <td className="font-medium text-foreground">{row.label}</td>
                      <td className={`uppercase ${statusTone(row.status)}`}>{row.status}</td>
                      <td className="text-muted-foreground">{row.value}</td>
                      <td>
                        {row.details.length === 0 ? (
                          <span className="text-muted-foreground">None</span>
                        ) : (
                          <details>
                            <summary>View details</summary>
                            <dl className="operator-detail-list">
                              {row.details.map((detail) => (
                                <div key={`${row.key}-${detail.label}`}>
                                  <dt>{detail.label}</dt>
                                  <dd>{detail.value}</dd>
                                </div>
                              ))}
                            </dl>
                          </details>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </section>
      ))}
    </div>
  );
}
