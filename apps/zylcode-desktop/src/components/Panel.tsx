// ---------------------------------------------------------------------------
// Shared panel primitives (Product Architecture v3 §24 — semantic tokens)
// ---------------------------------------------------------------------------

export function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <h2 className="text-xs font-semibold uppercase tracking-widest text-text-muted">
      {children}
    </h2>
  );
}

export function Panel({
  title,
  right,
  children,
  className = "",
}: {
  title?: string;
  right?: React.ReactNode;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div className={`border border-border rounded-lg bg-surface/50 ${className}`}>
      {title && (
        <div className="flex items-center justify-between px-3 py-2 border-b border-border">
          <SectionTitle>{title}</SectionTitle>
          {right}
        </div>
      )}
      <div className="p-3">{children}</div>
    </div>
  );
}
