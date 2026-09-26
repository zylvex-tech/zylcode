import { useEffect, useState } from "react";
import { StatusBadge, type CapabilityStatus } from "./CapabilityStatus";
import {
  fetchProjects,
  fetchArtifacts,
  fetchProofs,
  fetchModels,
  type ProjectsState,
  type ArtifactsState,
  type ProofsState,
  type ModelsState,
} from "../lib/foundation";

/**
 * Foundation-wave surfaces: the four persistence systems behind the product
 * (Project System, Artifact Bus, Proof Engine, model capability registry),
 * each rendered from its real service payload — including corrupt stores,
 * tampered artifacts, not-run proofs, and vision-unverified models.
 */
export const FoundationPanel: React.FC = () => {
  const [projects, setProjects] = useState<ProjectsState>({ kind: "loading" });
  const [artifacts, setArtifacts] = useState<ArtifactsState>({ kind: "loading" });
  const [proofs, setProofs] = useState<ProofsState>({ kind: "loading" });
  const [models, setModels] = useState<ModelsState>({ kind: "loading" });

  useEffect(() => {
    let cancelled = false;
    const load = () => {
      fetchProjects().then((v) => !cancelled && setProjects(v));
      fetchArtifacts().then((v) => !cancelled && setArtifacts(v));
      fetchProofs().then((v) => !cancelled && setProofs(v));
      fetchModels().then((v) => !cancelled && setModels(v));
    };
    load();
    const t = setInterval(load, 20000);
    return () => {
      cancelled = true;
      clearInterval(t);
    };
  }, []);

  const status = (s: { kind: string }): CapabilityStatus =>
    s.kind === "ready" ? "AVAILABLE" : s.kind === "loading" ? "LIMITED" : "BLOCKED";

  return (
    <div className="h-full p-3 overflow-y-auto space-y-3" data-testid="foundation-panel">
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
        {/* Project System */}
        <section className="border border-border rounded-lg p-3">
          <header className="flex items-center justify-between mb-2">
            <h4 className="text-sm font-medium">Project System</h4>
            <StatusBadge status={status(projects)} size="xs" />
          </header>
          {projects.kind === "loading" && (
            <p className="text-xs text-text-muted">Reading the project store…</p>
          )}
          {projects.kind === "unavailable" && (
            <p className="text-xs text-amber-400">{projects.reason}</p>
          )}
          {projects.kind === "ready" && (
            <div className="text-xs space-y-1">
              <p className="text-[11px] text-text-muted">
                Store state: <span className="font-mono text-text-secondary">{projects.storeState}</span>
              </p>
              {projects.projects.length === 0 ? (
                <p className="text-text-muted">
                  No projects registered yet. Projects persist in{" "}
                  <span className="font-mono">.zylcode/projects.json</span> and survive restarts.
                </p>
              ) : (
                projects.projects.map((p) => (
                  <div key={p.id} className="flex items-center gap-2">
                    <span className="font-mono text-text-secondary">{p.name}</span>
                    <span className="text-text-muted truncate">{p.root_path}</span>
                  </div>
                ))
              )}
            </div>
          )}
        </section>

        {/* Artifact Bus */}
        <section className="border border-border rounded-lg p-3">
          <header className="flex items-center justify-between mb-2">
            <h4 className="text-sm font-medium">Artifact Bus</h4>
            <StatusBadge status={status(artifacts)} size="xs" />
          </header>
          {artifacts.kind === "loading" && (
            <p className="text-xs text-text-muted">Reading artifact records…</p>
          )}
          {artifacts.kind === "unavailable" && (
            <p className="text-xs text-amber-400">{artifacts.reason}</p>
          )}
          {artifacts.kind === "ready" && (
            <div className="text-xs space-y-1">
              <p className="text-[11px] text-text-muted">
                Contract v{artifacts.contractVersion} — {artifacts.count} artifact
                {artifacts.count === 1 ? "" : "s"}; lifecycle is forward-only (proposed → generated →
                validated → reviewed → accepted → delivered).
              </p>
              {artifacts.artifacts.slice(0, 6).map((a) => {
                const kindLabel =
                  typeof a.kind === "string" ? a.kind : (a.kind as { Other: string }).Other;
                const tampered = a.verification_status === "tampered";
                return (
                  <div key={a.id} className="flex items-center gap-2">
                    <span className={tampered ? "text-red-400" : "text-emerald-400"}>
                      {tampered ? "✕" : "✓"}
                    </span>
                    <span className="font-mono text-text-secondary truncate max-w-[45%]">{a.id}</span>
                    <span className="text-text-muted">{kindLabel}</span>
                    <span className="ml-auto text-[10px] text-text-muted font-mono">{a.lifecycle}</span>
                  </div>
                );
              })}
            </div>
          )}
        </section>

        {/* Proof Engine */}
        <section className="border border-border rounded-lg p-3">
          <header className="flex items-center justify-between mb-2">
            <h4 className="text-sm font-medium">Proof Engine</h4>
            <StatusBadge status={status(proofs)} size="xs" />
          </header>
          {proofs.kind === "loading" && (
            <p className="text-xs text-text-muted">Reading proof records…</p>
          )}
          {proofs.kind === "unavailable" && (
            <p className="text-xs text-amber-400">{proofs.reason}</p>
          )}
          {proofs.kind === "ready" && (
            <div className="text-xs space-y-1">
              <p className="text-[11px] text-text-muted">
                Chain{" "}
                {proofs.chainIntact ? (
                  <span className="text-emerald-400">intact</span>
                ) : (
                  <span className="text-red-400 font-medium">BROKEN — history was modified</span>
                )}{" "}
                · {proofs.count} proof{proofs.count === 1 ? "" : "s"} · HEAD{" "}
                <span className="font-mono">{proofs.currentCommit || "?"}</span>
              </p>
              {proofs.proofs.slice(0, 6).map((p) => {
                const v = p.record.verification;
                const good = v === "passed";
                const neutral = v === "not_run" || v === "blocked";
                return (
                  <div key={p.record.id} className="flex items-center gap-2">
                    <span className={good ? "text-emerald-400" : neutral ? "text-amber-400" : "text-red-400"}>
                      {good ? "✓" : neutral ? "▲" : "✕"}
                    </span>
                    <span className="font-mono text-text-secondary truncate max-w-[40%]" title={p.record.command}>
                      {p.record.command}
                    </span>
                    <span className="text-text-muted">{v.replace(/_/g, " ")}</span>
                    {p.stale && (
                      <span className="ml-auto text-[10px] text-amber-400" title={`proof commit ${p.record.source_commit} ≠ HEAD`}>
                        stale
                      </span>
                    )}
                  </div>
                );
              })}
              <p className="text-[10px] text-text-muted pt-1">
                “Not run” and “blocked” are recorded observations — never converted to passes.
              </p>
            </div>
          )}
        </section>

        {/* Model capability registry */}
        <section className="border border-border rounded-lg p-3">
          <header className="flex items-center justify-between mb-2">
            <h4 className="text-sm font-medium">Model Capabilities</h4>
            <StatusBadge status={status(models)} size="xs" />
          </header>
          {models.kind === "loading" && (
            <p className="text-xs text-text-muted">Reading the model registry…</p>
          )}
          {models.kind === "unavailable" && (
            <p className="text-xs text-amber-400">{models.reason}</p>
          )}
          {models.kind === "ready" && (
            <div className="text-xs space-y-1">
              <p className="text-[11px] text-text-muted mb-1">{models.note}</p>
              {models.models.map((m) => (
                <div key={m.model_id} className="flex items-center gap-2">
                  <span className="font-mono text-text-secondary w-32 shrink-0 truncate" title={m.model_id}>
                    {m.model_id}
                  </span>
                  <span className="text-text-muted w-20 shrink-0">{m.provider}</span>
                  <span
                    className={
                      m.vision_interpretation === "vision_unverified"
                        ? "text-[10px] text-amber-400"
                        : "text-[10px] text-text-muted"
                    }
                    title="image_input is transport; interpretation needs a recorded test"
                  >
                    {m.vision_interpretation === "vision_unverified"
                      ? "VISION_UNVERIFIED"
                      : m.vision_interpretation}
                  </span>
                  {m.runtime_verified && (
                    <span className="ml-auto text-[10px] text-emerald-400">runtime ✓</span>
                  )}
                </div>
              ))}
            </div>
          )}
        </section>
      </div>
    </div>
  );
};

export default FoundationPanel;
