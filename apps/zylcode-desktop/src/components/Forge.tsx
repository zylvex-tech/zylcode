import { useMemo, useState } from "react";
import { FORGE_CATEGORIES, FORGE_FUTURE_PACKS, parseCapabilityPack } from "../lib/forge";
import { StatusBadge } from "./CapabilityStatus";
import { Panel } from "./Panel";

// ---------------------------------------------------------------------------
// Forge — capability marketplace shell (Product Architecture v3 §14–§19)
// Taxonomy metadata + truthful pack states + manifest validator + trust
// labels. NO fake plugins, NO checkout: commercial states render as domain
// information with "no commercial backend yet".
// ---------------------------------------------------------------------------

/** A concept pack derived from the taxonomy — explicitly NOT INSTALLED. */
type ConceptPack = {
  name: string;
  category: string;
  note: string;
};

const CONCEPT_PACKS: ConceptPack[] = FORGE_FUTURE_PACKS.map((name) => ({
  name,
  category: "taxonomy",
  note: "Concept — not implemented. Appears here as marketplace taxonomy metadata only.",
}));

/** Live validator: paste a manifest JSON and validate it against the schema. */
function ManifestValidator() {
  const [text, setText] = useState("");
  const [result, setResult] = useState<string[]>([]);

  function validate() {
    try {
      const parsed = JSON.parse(text);
      const res = parseCapabilityPack(parsed);
      setResult(res.ok ? ["VALID — pack accepted by the strict parser."] : res.errors);
    } catch (e) {
      setResult([`invalid JSON: ${String(e)}`]);
    }
  }

  return (
    <Panel title="MANIFEST VALIDATOR">
      <textarea
        value={text}
        onChange={(e) => setText(e.target.value)}
        rows={5}
        placeholder='{"id": "com.example.pack", ...}'
        className="w-full rounded-md border border-border bg-background px-2 py-1.5 font-mono text-[11px] outline-none focus:border-primary resize-none"
      />
      <button
        onClick={validate}
        className="mt-2 bg-secondary text-secondary-foreground px-3 py-1.5 rounded-md text-xs font-medium hover:bg-secondary-hover"
      >
        Validate manifest
      </button>
      {result.length > 0 && (
        <div className="mt-2 space-y-0.5">
          {result.map((r, i) => (
            <p
              key={i}
              className={`text-[11px] ${r.startsWith("VALID") ? "text-success" : "text-error"}`}
            >
              {r}
            </p>
          ))}
        </div>
      )}
    </Panel>
  );
}

export default function Forge() {
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState<string>("all");
  const [selectedConcept, setSelectedConcept] = useState<string | null>(null);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return CONCEPT_PACKS.filter(
      (p) =>
        (category === "all" || p.category === category) &&
        (q === "" || p.name.toLowerCase().includes(q)),
    );
  }, [query, category]);

  return (
    <div className="space-y-4">
      <Panel title="ZYLCODE FORGE">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <p className="text-xs text-text-muted max-w-2xl">
            A marketplace for capability packs — skills, commands, agents, MCP integrations, hooks,
            templates, knowledge, runtime adapters, tests, and evidence rules. The domain model,
            manifest schema, taxonomy, and trust labels are implemented; the commercial backend is
            not. Packs below are taxonomy concepts, explicitly marked NOT INSTALLED.
          </p>
          <StatusBadge status="LIMITED" />
        </div>
      </Panel>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-3">
        <div className="lg:col-span-2 space-y-3">
          <Panel title="BROWSE">
            <div className="flex flex-wrap gap-2 mb-3">
              <input
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Search capability packs…"
                className="flex-1 min-w-[180px] rounded-md border border-border bg-background px-3 py-1.5 text-sm outline-none focus:border-primary"
              />
              <select
                value={category}
                onChange={(e) => setCategory(e.target.value)}
                className="rounded-md border border-border bg-background px-2 py-1.5 text-sm outline-none focus:border-primary"
              >
                <option value="all">All categories</option>
                {FORGE_CATEGORIES.map((c) => (
                  <option key={c.slug} value={c.slug}>
                    {c.label}
                  </option>
                ))}
              </select>
            </div>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
              {filtered.map((p) => (
                <button
                  key={p.name}
                  onClick={() => setSelectedConcept(p.name)}
                  className={`text-left rounded border px-3 py-2 transition-colors ${
                    selectedConcept === p.name
                      ? "border-primary/50 bg-primary/5"
                      : "border-border hover:border-primary/30"
                  }`}
                >
                  <div className="flex items-center justify-between gap-2">
                    <span className="text-sm truncate">{p.name}</span>
                    <StatusBadge status="NOT INSTALLED" size="xs" />
                  </div>
                  <p className="text-[10px] text-text-muted mt-0.5 line-clamp-2">{p.note}</p>
                </button>
              ))}
              {filtered.length === 0 && (
                <p className="text-xs text-text-muted col-span-2">No concepts match the filter.</p>
              )}
            </div>
          </Panel>

          {selectedConcept && (
            <Panel title={`CONCEPT — ${selectedConcept}`}>
              <p className="text-xs text-text-muted">
                This is a marketplace taxonomy concept, not an implemented capability. When a real
                capability pack is published, its manifest (permissions, evidence, commercial model)
                will be displayed here before any installation decision.
              </p>
              <div className="mt-2 flex items-center gap-2">
                <StatusBadge status="NOT INSTALLED" />
                <span className="text-[11px] text-text-muted">
                  Trust labels are earned by evidence: UNVERIFIED → TESTED → VERIFIED → REPRODUCIBLE
                  → CERTIFIED.
                </span>
              </div>
            </Panel>
          )}
        </div>

        <div className="space-y-3">
          <ManifestValidator />
          <Panel title="TRUST MODEL">
            <div className="space-y-1 text-[11px] text-text-muted">
              <p>UNVERIFIED — no passing tests recorded.</p>
              <p>TESTED — tests pass, rung R2.</p>
              <p>VERIFIED — tests pass, rung R3 with verified commit.</p>
              <p>REPRODUCIBLE — rung R4.</p>
              <p>CERTIFIED — rung R5.</p>
              <p className="pt-1">
                Labels are computed from manifest evidence fields; a claimed rung above what
                evidence supports is clamped down.
              </p>
            </div>
          </Panel>
        </div>
      </div>
    </div>
  );
}
