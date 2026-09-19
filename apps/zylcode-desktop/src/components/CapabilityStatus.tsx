import type { ReactNode } from "react";

// ---------------------------------------------------------------------------
// Universal capability status language (Product Architecture v3 §8, §26)
// ---------------------------------------------------------------------------

export type CapabilityStatus =
  | "AVAILABLE"
  | "LIMITED"
  | "COMING SOON"
  | "BLOCKED"
  | "NOT INSTALLED";

const STATUS_STYLE: Record<CapabilityStatus, string> = {
  AVAILABLE:
    "bg-success/15 text-success border-success/30",
  LIMITED:
    "bg-warning/15 text-warning border-warning/30",
  "COMING SOON":
    "bg-primary/10 text-primary border-primary/30",
  BLOCKED:
    "bg-error/15 text-error border-error/30",
  "NOT INSTALLED":
    "bg-text-muted/10 text-text-muted border-text-muted/30",
};

const STATUS_DOT: Record<CapabilityStatus, string> = {
  AVAILABLE: "bg-success",
  LIMITED: "bg-warning",
  "COMING SOON": "bg-primary",
  BLOCKED: "bg-error",
  "NOT INSTALLED": "bg-text-muted",
};

export function StatusBadge({
  status,
  size = "sm",
}: {
  status: CapabilityStatus;
  size?: "sm" | "xs";
}) {
  return (
    <span
      className={`inline-flex items-center gap-1.5 border rounded-full font-medium tracking-wide uppercase ${
        size === "xs" ? "text-[10px] px-1.5 py-0" : "text-[11px] px-2 py-0.5"
      } ${STATUS_STYLE[status]}`}
    >
      <span className={`inline-block w-1.5 h-1.5 rounded-full ${STATUS_DOT[status]}`} />
      {status}
    </span>
  );
}

/** Proof rung chip for developer/evidence contexts only (v3 §26). */
export function RungBadge({ rung }: { rung: string }) {
  return (
    <span className="inline-flex items-center rounded border border-border bg-surface px-1.5 py-0 font-mono text-[10px] text-text-muted">
      {rung}
    </span>
  );
}

/** A disabled control with an explanatory status — never a fake control. */
export function StatusLockedAction({
  status,
  reason,
  children,
}: {
  status: CapabilityStatus;
  reason: string;
  children: ReactNode;
}) {
  return (
    <span className="inline-flex flex-col gap-0.5">
      <span
        aria-disabled="true"
        title={reason}
        className="inline-flex items-center justify-center gap-2 rounded-md border border-border bg-surface/60 px-4 py-2 text-sm font-medium text-text-muted cursor-not-allowed select-none"
      >
        {children}
        <StatusBadge status={status} size="xs" />
      </span>
      <span className="text-[11px] text-text-muted">{reason}</span>
    </span>
  );
}
