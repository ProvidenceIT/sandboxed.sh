"use client";

import { cn } from "@/lib/ui/cn";

export interface DataTableColumn {
  id: string;
  label?: string;
  width?: string;
}

export interface DataTableRow {
  [key: string]: unknown;
}

export interface DataTableProps {
  id: string;
  title?: string;
  columns: DataTableColumn[];
  rows: DataTableRow[];
  className?: string;
}

export function DataTable({
  id,
  title,
  columns,
  rows,
  className,
}: DataTableProps) {
  return (
    <div
      className={cn(
        "w-full max-w-2xl overflow-hidden rounded-lg border border-[var(--border)] bg-[var(--background-secondary)]",
        className
      )}
      data-slot="data-table"
      data-tool-ui-id={id}
    >
      {title && (
        <div className="border-b border-[var(--border)] px-4 py-2">
          <h3 className="text-sm font-medium text-[var(--foreground)]">{title}</h3>
        </div>
      )}
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-[var(--border)] bg-[var(--background-tertiary)]">
              {columns.map((col) => (
                <th
                  key={col.id}
                  className="px-4 py-2 text-left font-medium text-[var(--foreground-muted)]"
                  style={col.width ? { width: col.width } : undefined}
                >
                  {col.label ?? col.id}
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="divide-y divide-[var(--border)]">
            {rows.length === 0 ? (
              <tr>
                <td
                  colSpan={columns.length}
                  className="px-4 py-8 text-center text-[var(--foreground-muted)]"
                >
                  No data
                </td>
              </tr>
            ) : (
              rows.map((row, rowIndex) => (
                <tr key={rowIndex} className="hover:bg-[var(--background-tertiary)]">
                  {columns.map((col) => (
                    <td key={col.id} className="px-4 py-2 text-[var(--foreground)]">
                      {formatCellValue(row[col.id])}
                    </td>
                  ))}
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function formatCellValue(value: unknown): string {
  if (value === null || value === undefined) {
    return "-";
  }
  if (typeof value === "boolean") {
    return value ? "Yes" : "No";
  }
  if (typeof value === "number") {
    return value.toLocaleString();
  }
  if (typeof value === "object") {
    return JSON.stringify(value);
  }
  return String(value);
}

export interface SerializableDataTable {
  id: string;
  title?: string;
  columns: Array<{ id: string; label?: string; width?: string }>;
  rows: Array<Record<string, unknown>>;
}

export function parseSerializableDataTable(input: unknown): SerializableDataTable | null {
  if (!input || typeof input !== "object") return null;
  
  const obj = input as Record<string, unknown>;
  
  // Handle missing id by generating one
  const id = typeof obj.id === "string" ? obj.id : `table-${Date.now()}`;
  
  if (!Array.isArray(obj.columns) || !Array.isArray(obj.rows)) {
    return null;
  }
  
  // Parse columns - be more flexible with the format
  const columns: Array<{ id: string; label?: string; width?: string }> = [];
  
  for (const col of obj.columns) {
    if (typeof col === "string") {
      // Simple string column
      columns.push({ id: col, label: col });
    } else if (typeof col === "object" && col !== null) {
      const colObj = col as Record<string, unknown>;
      // Try different field names for ID
      const colId = String(colObj.id ?? colObj.key ?? colObj.field ?? colObj.name ?? colObj.header ?? "col");
      const label = String(colObj.label ?? colObj.header ?? colObj.title ?? colObj.name ?? colId);
      columns.push({ 
        id: colId, 
        label, 
        width: typeof colObj.width === "string" ? colObj.width : undefined 
      });
    }
  }
  
  if (columns.length === 0) return null;
  
  return {
    id,
    title: typeof obj.title === "string" ? obj.title : undefined,
    columns,
    rows: obj.rows as Array<Record<string, unknown>>,
  };
}
