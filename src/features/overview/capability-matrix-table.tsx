import { useMemo, useState } from "react";
import {
  flexRender,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
  type ColumnDef,
  type SortingState,
  type VisibilityState,
} from "@tanstack/react-table";
import type { ProviderInventorySummary } from "../../lib/types";

interface CapabilityMatrixTableProps {
  inventory: ProviderInventorySummary;
  isLoading: boolean;
}

interface CapabilityMatrixRow {
  providerId: string;
  accountId: string;
  displayName: string;
  authState: string;
  available: boolean;
  modelId: string;
  supportsTools: boolean;
  supportsStreaming: boolean;
  supportsReasoning: boolean;
  source: string;
}

const columns: ColumnDef<CapabilityMatrixRow>[] = [
  {
    accessorKey: "modelId",
    header: "Model",
  },
  {
    accessorKey: "providerId",
    header: "Provider",
  },
  {
    accessorKey: "displayName",
    header: "Account",
  },
  {
    accessorKey: "authState",
    header: "Auth State",
  },
  {
    accessorKey: "supportsTools",
    header: "Tools",
    cell: ({ row }) => (row.original.supportsTools ? "Yes" : "No"),
  },
  {
    accessorKey: "supportsStreaming",
    header: "Streaming",
    cell: ({ row }) => (row.original.supportsStreaming ? "Yes" : "No"),
  },
  {
    accessorKey: "supportsReasoning",
    header: "Reasoning",
    cell: ({ row }) => (row.original.supportsReasoning ? "Yes" : "No"),
  },
  {
    accessorKey: "source",
    header: "Source",
  },
];

function authStateTone(authState: string) {
  if (authState === "active") {
    return "text-emerald-600";
  }
  if (authState === "expired") {
    return "text-red-600";
  }
  return "text-amber-600";
}

export function CapabilityMatrixTable({ inventory, isLoading }: CapabilityMatrixTableProps) {
  const [sorting, setSorting] = useState<SortingState>([{ id: "modelId", desc: false }]);
  const [columnVisibility, setColumnVisibility] = useState<VisibilityState>({});
  const [showColumnPicker, setShowColumnPicker] = useState(false);
  const [search, setSearch] = useState("");
  const [providerScope, setProviderScope] = useState("all");
  const [accountScope, setAccountScope] = useState("all");

  const rows = useMemo(() => {
    const accountsById = new Map(
      inventory.accounts.map((account) => [account.account_id, account] as const),
    );

    return inventory.models.map((model) => {
      const account = accountsById.get(model.account_id);
      return {
        providerId: model.provider_id,
        accountId: model.account_id,
        displayName: account?.display_name ?? model.account_id,
        authState: account?.auth_state ?? "unknown",
        available: account?.available ?? false,
        modelId: model.model_id,
        supportsTools: model.supports_tools,
        supportsStreaming: model.supports_streaming,
        supportsReasoning: model.supports_reasoning,
        source: model.source,
      };
    });
  }, [inventory]);

  const filteredRows = useMemo(() => {
    const normalizedSearch = search.trim().toLowerCase();
    return rows.filter((row) => {
      if (providerScope !== "all" && row.providerId !== providerScope) {
        return false;
      }
      if (accountScope !== "all" && row.accountId !== accountScope) {
        return false;
      }
      if (!normalizedSearch) {
        return true;
      }
      return [
        row.modelId,
        row.providerId,
        row.displayName,
        row.accountId,
        row.authState,
        row.source,
      ]
        .join(" ")
        .toLowerCase()
        .includes(normalizedSearch);
    });
  }, [accountScope, providerScope, rows, search]);

  const providerOptions = useMemo(
    () => Array.from(new Set(rows.map((row) => row.providerId))).sort(),
    [rows],
  );
  const accountOptions = useMemo(
    () =>
      Array.from(
        new Map(rows.map((row) => [row.accountId, row.displayName] as const)).entries(),
      ).sort((left, right) => left[1].localeCompare(right[1])),
    [rows],
  );

  const table = useReactTable({
    data: filteredRows,
    columns,
    state: { sorting, columnVisibility },
    onSortingChange: setSorting,
    onColumnVisibilityChange: setColumnVisibility,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  });

  if (isLoading) {
    return <p>Loading capability inventory...</p>;
  }

  return (
    <section className="space-y-4" aria-labelledby="capability-matrix-heading">
      <div className="flex flex-wrap items-end gap-3">
        <label>
          Search
          <input
            aria-label="Capability matrix search"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
          />
        </label>
        <label>
          Provider scope
          <select
            aria-label="Provider scope"
            value={providerScope}
            onChange={(event) => setProviderScope(event.target.value)}
          >
            <option value="all">All providers</option>
            {providerOptions.map((providerId) => (
              <option key={providerId} value={providerId}>
                {providerId}
              </option>
            ))}
          </select>
        </label>
        <label>
          Account scope
          <select
            aria-label="Account scope"
            value={accountScope}
            onChange={(event) => setAccountScope(event.target.value)}
          >
            <option value="all">All accounts</option>
            {accountOptions.map(([accountId, displayName]) => (
              <option key={accountId} value={accountId}>
                {displayName}
              </option>
            ))}
          </select>
        </label>
        <button type="button" onClick={() => setShowColumnPicker((current) => !current)}>
          Columns
        </button>
      </div>
      {showColumnPicker ? (
        <div className="flex flex-wrap gap-3" aria-label="Capability matrix column visibility">
          {table.getAllLeafColumns().map((column) => (
            <label key={column.id}>
              <input
                type="checkbox"
                checked={column.getIsVisible()}
                onChange={column.getToggleVisibilityHandler()}
              />
              {String(column.columnDef.header)}
            </label>
          ))}
        </div>
      ) : null}
      <div className="overflow-x-auto rounded-xl border border-border/60 bg-background/60 p-4">
        <table className="min-w-full text-sm" aria-label="Capability matrix">
          <thead>
            {table.getHeaderGroups().map((headerGroup) => (
              <tr
                key={headerGroup.id}
                className="border-b border-border/60 text-left text-xs uppercase tracking-[0.16em] text-muted-foreground"
              >
                {headerGroup.headers.map((header) => (
                  <th key={header.id} className="px-2 py-2 font-medium">
                    {header.isPlaceholder ? null : (
                      <button
                        type="button"
                        onClick={header.column.getToggleSortingHandler()}
                        className="inline-flex items-center gap-1"
                      >
                        {flexRender(header.column.columnDef.header, header.getContext())}
                        {{
                          asc: "↑",
                          desc: "↓",
                        }[header.column.getIsSorted() as string] ?? null}
                      </button>
                    )}
                  </th>
                ))}
              </tr>
            ))}
          </thead>
          <tbody>
            {table.getRowModel().rows.map((row) => (
              <tr key={row.id} className="border-b border-border/40 align-top last:border-b-0">
                {row.getVisibleCells().map((cell) => (
                  <td key={cell.id} className="px-2 py-3">
                    {cell.column.id === "authState" ? (
                      <span className={authStateTone(row.original.authState)}>
                        {row.original.authState}
                        {!row.original.available ? " (degraded)" : ""}
                      </span>
                    ) : (
                      cell.column.columnDef.cell
                        ? flexRender(cell.column.columnDef.cell, cell.getContext())
                        : String(cell.getValue() ?? "")
                    )}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
        {table.getRowModel().rows.length === 0 ? (
          <p className="mt-3 text-sm text-muted-foreground">No capability rows match the current filters.</p>
        ) : null}
      </div>
    </section>
  );
}
