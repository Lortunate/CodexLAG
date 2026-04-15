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
    <div className="space-y-4">
      {sections.map((section) => (
        <section key={section.id} className="rounded-xl border border-border/60 bg-background/60 p-4">
          <div className="mb-3 flex items-start justify-between gap-4">
            <div>
              <h4 className="text-base font-semibold">{section.title}</h4>
              <p className="text-sm text-muted-foreground">{section.summary}</p>
            </div>
            <span className={`text-xs font-semibold uppercase tracking-[0.18em] ${statusTone(section.status)}`}>
              {section.status}
            </span>
          </div>
          {section.rows.length === 0 ? (
            <p className="text-sm text-muted-foreground">No rows recorded for this section.</p>
          ) : (
            <div className="overflow-x-auto">
              <table className="min-w-full text-sm" aria-label={section.title}>
                <thead>
                  <tr className="border-b border-border/60 text-left text-xs uppercase tracking-[0.16em] text-muted-foreground">
                    <th className="px-2 py-2 font-medium">Item</th>
                    <th className="px-2 py-2 font-medium">Status</th>
                    <th className="px-2 py-2 font-medium">Value</th>
                    <th className="px-2 py-2 font-medium">Details</th>
                  </tr>
                </thead>
                <tbody>
                  {section.rows.map((row) => (
                    <tr key={row.key} className="border-b border-border/40 align-top last:border-b-0">
                      <td className="px-2 py-3 font-medium">{row.label}</td>
                      <td className={`px-2 py-3 uppercase ${statusTone(row.status)}`}>{row.status}</td>
                      <td className="px-2 py-3">{row.value}</td>
                      <td className="px-2 py-3">
                        {row.details.length === 0 ? (
                          <span className="text-muted-foreground">None</span>
                        ) : (
                          <details>
                            <summary className="cursor-pointer text-muted-foreground">View details</summary>
                            <dl className="mt-2 space-y-2">
                              {row.details.map((detail) => (
                                <div key={`${row.key}-${detail.label}`}>
                                  <dt className="text-xs uppercase tracking-[0.14em] text-muted-foreground">
                                    {detail.label}
                                  </dt>
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
