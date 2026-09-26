import { useCallback, useEffect, useRef, useState } from "react";
import { StatusBadge, type CapabilityStatus } from "./CapabilityStatus";
import {
  fetchDeploy,
  fetchBuildState,
  startBuild,
  startPackage,
  type DeployState,
  type BuildState,
} from "../lib/delivery";

/**
 * Delivery surface: the real deploy targets, the real build pipeline
 * (triggerable), and real release packaging. Every state shown comes from
 * the service — a red pipeline is shown red, an uncommissioned target is
 * shown uncommissioned. Nothing here claims a deploy that did not happen.
 */
export const DeliveryPanel: React.FC = () => {
  const [deploy, setDeploy] = useState<DeployState>({ kind: "loading" });
  const [build, setBuild] = useState<BuildState>({ kind: "loading" });
  const [version, setVersion] = useState("");
  const [actionNote, setActionNote] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const pollRef = useRef<number | null>(null);

  const load = useCallback(() => {
    fetchDeploy().then(setDeploy);
    fetchBuildState().then(setBuild);
  }, []);

  useEffect(() => {
    load();
    const t = setInterval(load, 15000);
    return () => clearInterval(t);
  }, [load]);

  // While a run is in flight, poll faster so the panel reflects reality.
  useEffect(() => {
    if (build.kind === "ready" && build.state === "running") {
      pollRef.current = window.setInterval(() => {
        fetchBuildState().then(setBuild);
      }, 3000);
    }
    return () => {
      if (pollRef.current !== null) {
        clearInterval(pollRef.current);
        pollRef.current = null;
      }
    };
  }, [build]);

  const status = (s: { kind: string }): CapabilityStatus =>
    s.kind === "ready" ? "AVAILABLE" : s.kind === "loading" ? "LIMITED" : "BLOCKED";

  const buildStateBadge = (): CapabilityStatus => {
    if (build.kind !== "ready") return status(build);
    switch (build.state) {
      case "passed":
        return "AVAILABLE";
      case "running":
        return "LIMITED";
      case "failed":
        return "BLOCKED";
      default:
        return "COMING SOON"; // idle: pipeline exists, not yet run
    }
  };

  const onBuild = async () => {
    setBusy(true);
    setActionNote(null);
    const res = await startBuild();
    setActionNote(
      res.started
        ? res.note ?? "build pipeline launched"
        : res.error ?? "build could not start",
    );
    if (res.started) setTimeout(() => fetchBuildState().then(setBuild), 500);
    setBusy(false);
  };

  const onPackage = async (packageOnly: boolean) => {
    setBusy(true);
    setActionNote(null);
    const res = await startPackage(version, packageOnly);
    setActionNote(
      res.started
        ? res.note ?? "packaging pipeline launched"
        : res.error ?? "packaging could not start",
    );
    if (res.started) setTimeout(() => fetchBuildState().then(setBuild), 500);
    setBusy(false);
  };

  return (
    <div className="h-full p-3 overflow-y-auto space-y-3" data-testid="delivery-panel">
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
        {/* Build pipeline */}
        <section className="border border-border rounded-lg p-3">
          <header className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium">Build Pipeline</h3>
            <StatusBadge status={buildStateBadge()} />
          </header>
          {build.kind === "ready" ? (
            <>
              <p className="text-xs text-muted-foreground mb-2">
                state: <span className="font-mono">{build.state}</span>
                {build.error ? (
                  <span className="text-red-500"> — {build.error}</span>
                ) : null}
              </p>
              {build.steps.length > 0 ? (
                <ul className="text-xs space-y-1 mb-2">
                  {build.steps.map((s) => (
                    <li key={s.name} className="flex items-center gap-2">
                      <span
                        className={
                          s.passed
                            ? "text-green-500 font-mono"
                            : "text-red-500 font-mono"
                        }
                      >
                        {s.passed ? "\u2713" : "\u2717"}
                      </span>
                      <span className="font-mono">{s.name}</span>
                      {s.duration_ms !== null && s.duration_ms !== undefined ? (
                        <span className="text-muted-foreground">
                          {(s.duration_ms / 1000).toFixed(1)}s
                        </span>
                      ) : null}
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="text-xs text-muted-foreground mb-2">
                  {build.note ?? "no pipeline run in this service session"}
                </p>
              )}
              {build.packaged ? (
                <div className="text-xs border border-border rounded p-2 mb-2 font-mono">
                  <div>
                    release: {build.packaged.release_dir ?? "?"} (v
                    {build.packaged.version ?? "?"})
                  </div>
                  <div className="text-muted-foreground">
                    artifact {build.packaged.artifact_id ?? "?"} ·{" "}
                    {build.packaged.file_count ?? "?"} files · sha256{" "}
                    {(build.packaged.content_hash ?? "?").slice(0, 16)}
                  </div>
                </div>
              ) : null}
              <div className="flex flex-wrap items-center gap-2">
                <button
                  type="button"
                  onClick={onBuild}
                  disabled={busy || build.state === "running"}
                  className="text-xs px-2 py-1 rounded border border-border hover:bg-accent disabled:opacity-50"
                >
                  Run build pipeline
                </button>
                <input
                  type="text"
                  value={version}
                  onChange={(e) => setVersion(e.target.value)}
                  placeholder="version e.g. 0.3.0"
                  className="text-xs px-2 py-1 rounded border border-border bg-transparent w-40"
                />
                <button
                  type="button"
                  onClick={() => onPackage(false)}
                  disabled={busy || build.state === "running"}
                  className="text-xs px-2 py-1 rounded border border-border hover:bg-accent disabled:opacity-50"
                >
                  Package release
                </button>
                <button
                  type="button"
                  onClick={() => onPackage(true)}
                  disabled={busy || build.state === "running"}
                  className="text-xs px-2 py-1 rounded border border-border hover:bg-accent disabled:opacity-50"
                >
                  Package only
                </button>
              </div>
              {actionNote ? (
                <p className="text-xs text-muted-foreground mt-2">{actionNote}</p>
              ) : null}
            </>
          ) : (
            <p className="text-xs text-muted-foreground">
              {build.kind === "loading"
                ? "loading build status…"
                : `build status unavailable: ${build.reason}`}
            </p>
          )}
        </section>

        {/* Deploy targets */}
        <section className="border border-border rounded-lg p-3">
          <header className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-medium">Deploy Targets</h3>
            <StatusBadge status={status(deploy)} />
          </header>
          {deploy.kind === "ready" ? (
            <>
              <p className="text-xs text-muted-foreground mb-2">
                HEAD <span className="font-mono">{deploy.head || "?"}</span> — a
                target is commissioned only when its real infrastructure is
                configured; nothing here claims a deploy that did not happen.
              </p>
              <ul className="text-xs space-y-2">
                {(
                  [
                    ["GitHub Releases", deploy.github_release],
                    ["crates.io", deploy.crates_io],
                    ["Remote server", deploy.remote_server],
                  ] as const
                ).map(([label, t]) => (
                  <li key={label} className="border border-border rounded p-2">
                    <div className="flex items-center justify-between">
                      <span className="font-medium">{label}</span>
                      <span
                        className={`font-mono text-[10px] px-1.5 py-0.5 rounded ${
                          t.commissioned
                            ? "text-green-500 border border-green-700/40"
                            : "text-muted-foreground border border-border"
                        }`}
                      >
                        {t.state}
                      </span>
                    </div>
                    <p className="text-muted-foreground mt-1">{t.note}</p>
                  </li>
                ))}
              </ul>
            </>
          ) : (
            <p className="text-xs text-muted-foreground">
              {deploy.kind === "loading"
                ? "loading deploy targets…"
                : `deploy status unavailable: ${deploy.reason}`}
            </p>
          )}
        </section>
      </div>
    </div>
  );
};

export default DeliveryPanel;
