// ---------------------------------------------------------------------------
// Inspector (design/Inspector.tsx)
//
// Two subjects, and it never mixes them up:
//   * the selected component — its source location, props, tokens and structure;
//   * the selected rendered element — what the browser actually computed, and
//     which tokens explain those values.
//
// Prop edits are validated against the prop's own type before they reach the
// canvas: an enum cannot take a value outside its union, a number must be
// finite, and a prop the canvas cannot construct is shown read-only with the
// reason, not silently dropped.
// ---------------------------------------------------------------------------

import { useEffect, useState } from "react";
import {
  baselineKey,
  clearBaseline,
  diffSnapshots,
  readBaseline,
  saveBaseline,
  toBaseline,
  type BaselineSnapshot,
} from "./baseline";
import {
  a11yFindings,
  componentChildren,
  coercePropValue,
  mountPlan,
  parentComponents,
} from "./catalog";
import type { CanvasSelection } from "./DesignCanvas";
import { SourcePane } from "./SourcePane";
import type { ComponentEntry, DesignIndex, PropSpec } from "./types";

export type InspectorTab = "component" | "element" | "structure" | "source";

interface InspectorProps {
  index: DesignIndex;
  entry: ComponentEntry | null;
  theme: string;
  props: Record<string, unknown>;
  onPropChange: (name: string, value: unknown) => void;
  selection: CanvasSelection | null;
  onSelectComponent: (id: string) => void;
  defaultTab?: InspectorTab;
}

function isEditable(prop: PropSpec): boolean {
  if (prop.kind === "node") return prop.name === "children";
  return (
    prop.kind === "enum" ||
    prop.kind === "boolean" ||
    prop.kind === "string" ||
    prop.kind === "number"
  );
}

function PropEditor({
  prop,
  value,
  onChange,
}: {
  prop: PropSpec;
  value: unknown;
  onChange: (name: string, value: unknown) => void;
}) {
  const [draft, setDraft] = useState(String(value ?? ""));
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setDraft(String(value ?? ""));
    setError(null);
  }, [value, prop.name]);

  const commit = (raw: string) => {
    const result = coercePropValue(prop, raw);
    if (result.ok) {
      setError(null);
      onChange(prop.name, result.value);
    } else {
      setError(result.reason);
    }
  };

  return (
    <div className="py-1">
      <div className="flex items-baseline justify-between gap-2">
        <span className="font-mono text-[11px] text-text-primary">
          {prop.name}
          <span className="ml-1 text-text-muted/70">{prop.required ? "*" : ""}</span>
        </span>
        <span
          className="max-w-[55%] truncate font-mono text-[10px] text-text-muted"
          title={prop.type}
        >
          {prop.type}
        </span>
      </div>

      {!isEditable(prop) ? (
        <p className="mt-0.5 text-[10px] text-text-muted">
          not editable here — supplied by the app
        </p>
      ) : prop.kind === "enum" && prop.values ? (
        <select
          value={String(value ?? "")}
          onChange={(e) => commit(e.target.value)}
          aria-label={`Value for ${prop.name}`}
          className="mt-1 w-full rounded border border-border bg-background px-1 py-0.5 font-mono text-[11px] text-text-primary"
        >
          {prop.values.map((option) => (
            <option key={option} value={option}>
              {option}
            </option>
          ))}
        </select>
      ) : prop.kind === "boolean" ? (
        <label className="mt-1 flex items-center gap-1.5 text-[11px] text-text-secondary">
          <input
            type="checkbox"
            checked={value === true}
            onChange={(e) => onChange(prop.name, e.target.checked)}
            aria-label={`Toggle ${prop.name}`}
          />
          {value === true ? "true" : "false"}
        </label>
      ) : (
        <input
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={() => commit(draft)}
          onKeyDown={(e) => {
            if (e.key === "Enter") commit(draft);
            if (e.key === "Escape") {
              setDraft(String(value ?? ""));
              setError(null);
            }
          }}
          aria-label={`Value for ${prop.name}`}
          className="mt-1 w-full rounded border border-border bg-background px-1 py-0.5 font-mono text-[11px] text-text-primary"
        />
      )}

      {error && <p className="mt-1 text-[10px] text-error">{error}</p>}
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="border-b border-border px-3 py-2">
      <h3 className="text-[10px] font-mono uppercase tracking-[0.16em] text-text-muted">{title}</h3>
      <div className="mt-1.5">{children}</div>
    </section>
  );
}

function TokenChips({ tokens }: { tokens: string[] }) {
  if (tokens.length === 0)
    return <p className="text-[11px] text-text-muted">no token utility in this source</p>;
  return (
    <div className="flex flex-wrap gap-1">
      {tokens.map((token) => (
        <span
          key={token}
          className="rounded border border-border px-1.5 py-[1px] font-mono text-[10px] text-text-secondary"
        >
          {token}
        </span>
      ))}
    </div>
  );
}

export function Inspector({
  index,
  entry,
  theme,
  props,
  onPropChange,
  selection,
  onSelectComponent,
  defaultTab = "component",
}: InspectorProps) {
  const [tab, setTab] = useState<InspectorTab>(defaultTab);
  const [baseline, setBaseline] = useState<BaselineSnapshot | null>(null);

  useEffect(() => {
    setTab(defaultTab);
  }, [entry?.id, defaultTab]);

  useEffect(() => {
    setBaseline(
      selection && entry
        ? readBaseline(baselineKey(entry.id, selection.tag, selection.classes))
        : null,
    );
  }, [entry, selection]);

  if (!entry) {
    return <p className="p-3 text-xs text-text-muted">No component selected.</p>;
  }

  const plan = mountPlan(entry);
  const findings = a11yFindings(entry);
  const children = componentChildren(index, entry);
  const parents = parentComponents(index, entry);
  const tokens = [...new Set(entry.tokens)].sort();

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex flex-wrap gap-1 border-b border-border px-2 py-1.5">
        {(["component", "element", "structure", "source"] as InspectorTab[]).map((id) => (
          <button
            key={id}
            type="button"
            onClick={() => setTab(id)}
            className={`rounded border px-2 py-0.5 text-[10px] font-mono ${
              tab === id ? "border-primary text-primary" : "border-border text-text-muted"
            }`}
          >
            {id}
          </button>
        ))}
        <span className="ml-auto font-mono text-[10px] text-text-muted">{theme}</span>
      </div>

      <div className="min-h-0 flex-1 overflow-auto">
        {tab === "component" && (
          <>
            <Section title="identity">
              <p className="text-sm text-text-primary">{entry.name}</p>
              <p className="mt-0.5 font-mono text-[10px] text-text-muted">
                {entry.file}:{entry.line} · {entry.kind}
                {entry.extends.length > 0 ? ` · extends ${entry.extends.join(", ")}` : ""}
              </p>
              <p
                className={`mt-1 font-mono text-[10px] ${plan.mountable ? "text-success" : "text-warning"}`}
                data-testid="mount-status"
              >
                {plan.mountable ? "MOUNTABLE in canvas" : `NOT MOUNTABLE — ${plan.reason}`}
              </p>
            </Section>

            <Section
              title={`props (${entry.props.length}${entry.propsUnknown ? " · type unresolved" : ""})`}
            >
              {entry.props.length === 0 ? (
                <p className="text-[11px] text-text-muted">
                  {entry.propsUnknown
                    ? "the props type did not resolve"
                    : "this component takes no props"}
                </p>
              ) : (
                <div className="divide-y divide-border/50">
                  {entry.props.map((prop) => (
                    <PropEditor
                      key={prop.name}
                      prop={prop}
                      value={props[prop.name]}
                      onChange={onPropChange}
                    />
                  ))}
                </div>
              )}
            </Section>

            <Section title={`tokens (${tokens.length})`}>
              <TokenChips tokens={tokens} />
            </Section>

            <Section title="accessibility">
              <ul className="space-y-1">
                {findings.map((finding) => (
                  <li key={finding.rule} className="flex gap-2 text-[11px]">
                    <span
                      className={`shrink-0 font-mono text-[9px] ${
                        finding.verdict === "PASS"
                          ? "text-success"
                          : finding.verdict === "WARNING"
                            ? "text-warning"
                            : "text-text-muted"
                      }`}
                    >
                      {finding.verdict}
                    </span>
                    <span className="text-text-secondary">
                      <span className="font-mono text-text-muted">{finding.rule}</span> —{" "}
                      {finding.detail}
                    </span>
                  </li>
                ))}
              </ul>
            </Section>
          </>
        )}

        {tab === "element" && (
          <>
            {!selection ? (
              <p className="p-3 text-xs text-text-muted">
                Click an element inside the canvas frame to inspect what the browser computed for
                it.
              </p>
            ) : (
              <>
                <Section title="rendered element">
                  <p className="font-mono text-[11px] text-text-primary">&lt;{selection.tag}&gt;</p>
                  <p className="mt-0.5 text-[10px] text-text-muted">
                    depth {selection.depth} from the frame root
                  </p>
                  {selection.text && (
                    <p
                      className="mt-1 truncate text-[11px] text-text-secondary"
                      title={selection.text}
                    >
                      “{selection.text}”
                    </p>
                  )}
                  {selection.classes && (
                    <p className="mt-1 break-words font-mono text-[10px] text-text-muted">
                      {selection.classes}
                    </p>
                  )}
                </Section>

                <Section title="attributed tokens">
                  <TokenChips tokens={selection.tokens} />
                  {selection.tokens.length === 0 && (
                    <p className="mt-1 text-[10px] text-text-muted">
                      No theme token matches these computed values — they may come from a literal or
                      a component class.
                    </p>
                  )}
                </Section>

                <Section title="computed style">
                  <table className="w-full text-left font-mono text-[10px]">
                    <tbody>
                      {Object.entries(selection.snapshot)
                        .filter(([, value]) => Boolean(value))
                        .map(([property, value]) => (
                          <tr key={property} className="border-t border-border/40">
                            <td className="py-0.5 pr-2 text-text-muted">{property}</td>
                            <td className="py-0.5 text-text-primary">{value}</td>
                          </tr>
                        ))}
                    </tbody>
                  </table>
                </Section>

                <Section title="baseline comparison">
                  <p className="text-[10px] text-text-muted">
                    Captures this element&apos;s computed style for this component, theme and
                    viewport. The comparison is property by property; it is not a pixel diff.
                  </p>
                  <div className="mt-1.5 flex flex-wrap gap-1">
                    <button
                      type="button"
                      className="rounded border border-border px-2 py-0.5 text-[10px] font-mono text-text-secondary"
                      onClick={() => {
                        if (!entry) return;
                        const key = baselineKey(entry.id, selection.tag, selection.classes);
                        const next = toBaseline(selection.snapshot, theme, 0, selection.tokens);
                        saveBaseline(key, next);
                        setBaseline(next);
                      }}
                    >
                      capture baseline
                    </button>
                    <button
                      type="button"
                      className="rounded border border-border px-2 py-0.5 text-[10px] font-mono text-text-muted"
                      onClick={() => {
                        if (!entry) return;
                        clearBaseline(baselineKey(entry.id, selection.tag, selection.classes));
                        setBaseline(null);
                      }}
                    >
                      clear
                    </button>
                  </div>

                  {!baseline ? (
                    <p className="mt-1.5 text-[10px] text-text-muted">
                      no baseline captured for this element
                    </p>
                  ) : (
                    <div className="mt-1.5" data-testid="baseline-diff">
                      <p className="text-[10px] text-text-muted">
                        captured {baseline.capturedAt.slice(0, 19).replace("T", " ")} · theme{" "}
                        {baseline.theme}
                      </p>
                      <table className="mt-1 w-full text-left font-mono text-[10px]">
                        <tbody>
                          {diffSnapshots(baseline, selection.snapshot).map((diff) => (
                            <tr key={diff.property} className="border-t border-border/40">
                              <td className="py-0.5 pr-2 text-text-muted">{diff.property}</td>
                              <td
                                className={`py-0.5 ${diff.changed ? "text-warning" : "text-text-secondary"}`}
                              >
                                {diff.changed
                                  ? `${diff.baseline || "—"} → ${diff.current || "—"}`
                                  : "unchanged"}
                              </td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  )}
                </Section>
              </>
            )}
          </>
        )}

        {tab === "structure" && (
          <>
            <Section title={`renders (${children.length})`}>
              {children.length === 0 ? (
                <p className="text-[11px] text-text-muted">
                  no indexed component is rendered directly here
                </p>
              ) : (
                <ul className="space-y-1">
                  {children.map((child) => (
                    <li key={child.id}>
                      <button
                        type="button"
                        onClick={() => onSelectComponent(child.id)}
                        className="flex w-full items-baseline justify-between gap-2 rounded border border-border/60 px-2 py-1 text-left hover:border-primary/50"
                      >
                        <span className="text-[11px] text-text-primary">{child.name}</span>
                        <span className="font-mono text-[10px] text-text-muted">{child.kind}</span>
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </Section>

            <Section title={`rendered by (${parents.length})`}>
              {parents.length === 0 ? (
                <p className="text-[11px] text-text-muted">
                  no indexed component renders this one directly
                </p>
              ) : (
                <ul className="space-y-1">
                  {parents.map((parent) => (
                    <li key={parent.id}>
                      <button
                        type="button"
                        onClick={() => onSelectComponent(parent.id)}
                        className="flex w-full items-baseline justify-between gap-2 rounded border border-border/60 px-2 py-1 text-left hover:border-primary/50"
                      >
                        <span className="text-[11px] text-text-primary">{parent.name}</span>
                        <span className="font-mono text-[10px] text-text-muted">{parent.kind}</span>
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </Section>

            <Section title={`classes (${new Set(entry.classes).size} distinct)`}>
              <p className="break-words font-mono text-[10px] text-text-muted">
                {[...new Set(entry.classes)].join(" ") || "—"}
              </p>
            </Section>
          </>
        )}

        {tab === "source" && <SourcePane file={entry.file} line={entry.line} />}
      </div>
    </div>
  );
}
