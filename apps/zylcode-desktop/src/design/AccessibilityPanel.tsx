// ---------------------------------------------------------------------------
// Accessibility panel (design/AccessibilityPanel.tsx)
//
// Two different kinds of evidence, kept apart:
//   * per component, static findings from the source the indexer read — these
//     can be PASS, WARNING or UNKNOWN, and the UNKNOWN rows say what would
//     settle them;
//   * per theme, WCAG contrast computed from the actual token values. A token
//     that is not a literal colour is UNKNOWN, never estimated.
//
// Nothing here runs an axe pass or claims a screen-reader result. The panel
// says which of those it is.
// ---------------------------------------------------------------------------

import { useMemo, useState } from "react";
import { a11yFindings, contrastMatrix } from "./catalog";
import type { ComponentEntry, DesignIndex, Verdict } from "./types";

interface AccessibilityPanelProps {
  index: DesignIndex;
  entry: ComponentEntry | null;
  theme: string;
}

function tone(verdict: Verdict): string {
  if (verdict === "PASS") return "text-success";
  if (verdict === "WARNING") return "text-warning";
  return "text-text-muted";
}

export function AccessibilityPanel({ index, entry, theme }: AccessibilityPanelProps) {
  const [allThemes, setAllThemes] = useState(true);
  const findings = entry ? a11yFindings(entry) : [];

  const rows = useMemo(() => {
    const themes = allThemes ? index.themes.map((t) => t.id) : [theme];
    return themes.flatMap((id) => contrastMatrix(index, id).map((row) => ({ theme: id, ...row })));
  }, [index, theme, allThemes]);

  const checked = rows.filter((r) => r.verdict !== "UNKNOWN");
  const failing = checked.filter((r) => r.verdict === "WARNING");

  return (
    <div className="min-h-0 flex-1 overflow-auto">
      <section className="border-b border-border px-3 py-2">
        <h3 className="text-[10px] font-mono uppercase tracking-[0.16em] text-text-muted">
          component findings
        </h3>
        {!entry ? (
          <p className="mt-1 text-xs text-text-muted">Select a component to audit its source.</p>
        ) : (
          <>
            <p className="mt-1 font-mono text-[11px] text-text-primary">
              {entry.name} · {entry.file}:{entry.line}
            </p>
            <ul className="mt-1.5 space-y-1">
              {findings.map((finding) => (
                <li key={finding.rule} className="flex items-start gap-2 text-[11px]">
                  <span className={`w-16 shrink-0 font-mono text-[9px] ${tone(finding.verdict)}`}>
                    {finding.verdict}
                  </span>
                  <span className="text-text-secondary">
                    <span className="font-mono text-text-muted">{finding.rule}</span> —{" "}
                    {finding.detail}
                  </span>
                </li>
              ))}
            </ul>
            <p className="mt-2 text-[10px] text-text-muted">
              Static analysis of the component&apos;s source. A keyboard pass, a screen-reader pass
              and an axe run are not part of this panel; UNKNOWN rows are the checks those passes
              would settle.
            </p>
          </>
        )}
      </section>

      <section className="px-3 py-2">
        <div className="flex items-center justify-between gap-2">
          <h3 className="text-[10px] font-mono uppercase tracking-[0.16em] text-text-muted">
            contrast (WCAG AA, {checked.length} pairs measured)
          </h3>
          <button
            type="button"
            onClick={() => setAllThemes(!allThemes)}
            className="rounded border border-border px-2 py-0.5 text-[10px] font-mono text-text-muted"
          >
            {allThemes ? `only ${theme}` : "all themes"}
          </button>
        </div>

        <p className="mt-1 text-[10px] text-text-muted">
          {failing.length === 0
            ? "every measured pair meets 4.5:1 for body text."
            : `${failing.length} pair(s) fall below AA: ${[...new Set(failing.map((f) => `${f.foreground} on ${f.background}`))].join(", ")}.`}
        </p>

        <table className="mt-2 w-full text-left font-mono text-[10px]">
          <thead className="text-[9px] uppercase tracking-wider text-text-muted">
            <tr>
              <th className="py-1 pr-2 font-medium">theme</th>
              <th className="py-1 pr-2 font-medium">pair</th>
              <th className="py-1 pr-2 text-right font-medium">ratio</th>
              <th className="py-1 text-right font-medium">aa</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr
                key={`${row.theme}:${row.foreground}:${row.background}`}
                className="border-t border-border/40"
              >
                <td className="py-0.5 pr-2 text-text-muted">{row.theme}</td>
                <td className="py-0.5 pr-2 text-text-secondary">
                  {row.foreground.replace("--color-", "")} on{" "}
                  {row.background.replace("--color-", "")}
                </td>
                <td className="py-0.5 pr-2 text-right text-text-primary">
                  {row.ratio === null ? "—" : `${row.ratio.toFixed(2)}:1`}
                </td>
                <td className={`py-0.5 text-right ${tone(row.verdict)}`}>{row.verdict}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </div>
  );
}
