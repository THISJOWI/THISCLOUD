"use client";

import { Resource } from "@/lib/api";

/* ---- StatusBadge ---- */
export function StatusBadge({ status }: { status: string }) {
  const s = status?.toLowerCase() ?? "";
  let cls = "badge-info";
  if (s === "running" || s === "active" || s === "online") cls = "badge-running";
  else if (s === "stopped" || s === "error" || s === "offline" || s === "failed") cls = "badge-stopped";
  else if (s === "pending" || s === "creating" || s === "deleting") cls = "badge-pending";

  return (
    <span className={`badge ${cls}`}>
      <span className="badge-dot" />
      {status}
    </span>
  );
}

/* ---- StatCard ---- */
export function StatCard({
  label,
  value,
  icon,
  accent,
}: {
  label: string;
  value: string | number;
  icon?: string;
  accent?: string;
}) {
  return (
    <div className="stat-card">
      <div className="stat-card-header">
        <span className="stat-label">{label}</span>
        {icon && (
          <div
            className="stat-icon"
            style={{
              background: `color-mix(in srgb, ${accent ?? "var(--accent)"} 15%, transparent)`,
              color: accent ?? "var(--accent)",
            }}
          >
            {icon}
          </div>
        )}
      </div>
      <div className="stat-value" style={{ color: accent ?? "var(--fg)" }}>
        {value}
      </div>
    </div>
  );
}

/* ---- ResourceTable ---- */
export function ResourceTable({
  title,
  headers,
  rows,
  onDelete,
  emptyText,
}: {
  title: string;
  headers: string[];
  rows: Resource[];
  onDelete?: (type: string, id: string) => void;
  emptyText?: string;
}) {
  const hasActions = !!onDelete;

  function formatCell(h: string, row: Resource) {
    const val = row[h];
    if (h === "status") return <StatusBadge status={String(val ?? "")} />;
    if (h === "id") return <span className="id-cell">{String(val ?? "")}</span>;
    if (Array.isArray(val)) {
      const arr = val as unknown[];
      if (arr.length === 0) return <span className="text-muted">—</span>;
      return (
        <span className="id-cell" style={{ maxWidth: 200 }}>
          {arr.join(", ")}
        </span>
      );
    }
    if (h === "image" && !val) return <span className="text-muted">—</span>;
    return String(val ?? "");
  }

  return (
    <div className="table-wrap">
      <div className="table-toolbar">
        <span className="table-title">{title}</span>
        <div className="table-actions">
          <span className="text-muted" style={{ fontSize: 12 }}>
            {rows.length} item{rows.length !== 1 ? "s" : ""}
          </span>
        </div>
      </div>
      <table>
        <thead>
          <tr>
            {headers.map((h) => (
              <th key={h}>{h}</th>
            ))}
            {hasActions && <th style={{ width: 60, textAlign: "center" }}>Actions</th>}
          </tr>
        </thead>
        <tbody>
          {rows.length === 0 && (
            <tr>
              <td colSpan={headers.length + (hasActions ? 1 : 0)} className="empty">
                {emptyText ?? "No resources"}
              </td>
            </tr>
          )}
          {rows.map((r) => (
            <tr key={r.id}>
              {headers.map((h) => (
                <td key={h}>{formatCell(h, r)}</td>
              ))}
              {hasActions && (
                <td style={{ textAlign: "center" }}>
                  <button
                    onClick={() => onDelete!(String(r.type), String(r.id))}
                    className="btn btn-danger btn-ghost"
                    title={`Delete ${r.type}:${r.id}`}
                    style={{ fontSize: 11, padding: "2px 6px" }}
                  >
                    Delete
                  </button>
                </td>
              )}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
