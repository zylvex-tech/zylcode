import { useMemo, useState } from "react";
import {
  FORGE_CATEGORIES,
  FORGE_FUTURE_PACKS,
  parseCapabilityPack,
  trustLabelOf,
  type CapabilityPack,
} from "../lib/forge";
import { StatusBadge, RungBadge } from "./CapabilityStatus";
import { Panel, SectionTitle } from "./Panel";

// ---------------------------------------------------------------------------
// Forge — capability marketplace shell (Product Architecture v3 §14–§19)
// Taxonomy metadata + truthful pack states + manifest validator + trust
// labels. NO fake plugins, NO checkout: commercial states render as domain
// information with "no commercial backend yet".
// ---------------------------------------------------------------------------

const TRUST_LABEL_STYLE: Record<string, string> = {
  UNVERIFIED: "text-text-muted border-text-muted/40",
  TESTED: "text-warning border-warning/40",
  VERIFIED: "text-success border-success/40",
  REPRODUCIBLE: "text-success border-success/60",
  CERTIFIED: "text-primary border-primary/60",
};

const PERMISSION_LABEL: Record<string, string> = {
  none: "None",
  read: "Read",
  write: "Write",
  "read-write": "Read/Write",
  execute: "Execute",
  "read-commit": "Read/Commit",
  control: "Control",
};

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

function PackDetail({ pack }: { pack: CapabilityPack }) {
  const label = trustLabelOf(pack);
  return (
    <Panel title={`PACK — ${pack.name}`}>
      <div className="space-y-3 text-xs">
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-mono text-[11px] text-text-muted">{pack.id}</span>
          <span className="font-mono text-[11px] text-text-muted">v{pack.version}</span>
          <span
            className={`inline-flex items-center rounded border px-1.5 py-0 text-[10px] font-medium tracking-wide uppercase ${TRUST_LABEL_STYLE[label]}`}
          >
            {label}
          </span>
          <RungBadge rung={pack.evidence.verification_rung} />
        </div>
        <p className="text-text-muted">{pack.description || "No description."}</p>

        <div>
          <SectionTitle>PERMISSIONS</SectionTitle>
          <div className="mt-1 grid grid-cols-2 sm:grid-cols-4 gap-1.5">
            <span className="rounded border border-border px-2 py-1">
              FILESYSTEM <b className="font-mono">{PERMISSION_LABEL[pack.permissions.filesystem]}</b>
            </span>
            <span className="rounded border border-border px-2 py-1">
              SHELL <b className="font-mono">{PERMISSION_LABEL[pack.permissions.shell]}</b>
            </span>
            <span className="rounded border border-border px-2 py-1">
              GIT <b className="font-mono">{PERMISSION_LABEL[pack.permissions.git]}</b>
            </span>
            <span className="rounded border border-border px-2 py-1">
              BROWSER <b className="font-mono">{PERMISSION_LABEL[pack.permissions.browser]}</b>
            </span>
            <span className="rounded border border-border px-2 py-1 col-span-2">
              NETWORK{" "}
              <b className="font-mono">
                {pack.permissions.network.length > 0 ? pack.permissions.network.join(", ") : "none"}
              </b>
            </span>
            <span className="rounded border border-border px-2 py-1 col-span-2">
              SECRETS{" "}
              <b className="font-mono">
                {pack.permissions.secrets.length > 0 ? pack.permissions.secrets.join(", ") : "none"}
              </b>
            </span>
          </div>
        </div>

        <div>
          <SectionTitle>EVIDENCE</SectionTitle>
          <div className="mt-1 space-y-0.5 font-mono text-[11px] text-text-muted">
            <p>test_status: {pack.evidence.test_status}</p>
            <p>verification_rung: {pack.evidence.verification_rung}</p>
            <p>verified_commit: {pack.evidence.verified_commit ?? "NOT CAPTURED"}</p>
            <p>
              platforms_verified:{" "}
              {pack.evidence.platforms_verified.length > 0
                ? pack.evidence.platforms_verified.join(", ")
                : "NOT CAPTURED"}
            </p>
          </div>
        </div>

        <div>
          <SectionTitle>COMMERCIAL</SectionTitle>
          <p className="mt-1 text-[11px] text-text-muted">
            {pack.commercial.model} — no commercial backend is implemented; purchasing is not
            possible. Installation requires explicit user action and is not available for concept
            packs.
          </p>
        </div>

        <div className="flex flex-col gap-1.5">
          <span
            aria-disabled="true"
            className="inline-flex items-center justify-center gap-2 rounded-md border border-border bg-surface/60 px-4 py-2 text-sm font-medium text-text-muted cursor-not-allowed select-none"
          >
            Install <StatusBadge status="NOT INSTALLED" size="xs" />
          </span>
          <span
            aria-disabled="true"
            className="inline-flex items-center justify-center gap-2 rounded-md border border-border bg-surface/60 px-4 py-2 text-sm font-medium text-text-muted cursor-not-allowed select-none"
          >
            Try Demo <StatusBadge status="COMING SOON" size="xs" />
          </span>
          <span className="text-[10px] text-text-muted/80">
            DEMO ENGINE — PROPOSED. Demo missions will execute real functionality once the demo
            engine exists; no simulated success is shown today.
          </span>
        </div>
      </div>
    </Panel>
  );
}

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
            <p key={i} className={`text-[11px] ${r.startsWith("VALID") ? "text-success" : "text-error"}`}>
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
                Labels are computed from manifest evidence fields; a claimed rung above what evidence
                supports is clamped down.
              </p>
            </div>
          </Panel>
        </div>
      </div>
    </div>
  );
}
