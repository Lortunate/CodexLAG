import type { DiagnosticsSection } from "../../lib/types";

interface DiagnosticsTableProps {
  sections: DiagnosticsSection[];
}

function statusTone(status: string) {
  switch (status) {
    case "error":
      return "text-red-600";
    case "warn":
      return "text-amber-600";
    case "healthy":
      return "text-emerald-600";
    default:
      return "text-muted-foreground";
  }
}

export function DiagnosticsTable({ sections }: DiagnosticsTableProps) {
  if (sections.length === 0) {
    return <p>No diagnostics have been captured yet.</p>;
  }

  return (
    <div className="diagnostics-stack">
      {sections.map((section) => (
        <section key={section.id} className="panel">
          <div className="diagnostics-header">
            <div>
              <h4>{section.title}</h4>
              <p className="panel-intro">{section.summary}</p>
            </div>
            <span className={`diagnostics-status ${statusTone(section.status)}`}>
              {section.status}
            </span>
          </div>
          {section.rows.length === 0 ? (
            <p className="panel-intro">No rows recorded for this section.</p>
          ) : (
            <div className="table-shell">
              <table aria-label={section.title}>
                <thead>
                  <tr>
                    <th scope="col">Item</th>
                    <th scope="col">Status</th>
                    <th scope="col">Value</th>
                    <th scope="col">Details</th>
                  </tr>
                </thead>
                <tbody>
                  {section.rows.map((row) => (
                    <tr key={row.key}>
                      <td>{row.label}</td>
                      <td className={statusTone(row.status)}>{row.status}</td>
                      <td>{row.value}</td>
                      <td>
                        {row.details.length === 0 ? (
                          <span className="panel-intro">None</span>
                        ) : (
                          <details>
                            <summary>View details</summary>
                            <dl className="diagnostics-detail-list">
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
