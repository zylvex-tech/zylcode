import { Panel } from "./Panel";
import { StatusBadge } from "./CapabilityStatus";
import { IS_DESKTOP } from "../lib/runtime";

export function RuntimeLab() {
  const targets: { name: string; status: "AVAILABLE" | "LIMITED" | "NOT INSTALLED"; note: string }[] = [
    { name: "WEB", status: "AVAILABLE", note: "Vite dev/preview of the current project" },
    {
      name: "DESKTOP",
      status: IS_DESKTOP ? "AVAILABLE" : "LIMITED",
      note: IS_DESKTOP ? "Tauri shell active" : "Requires the Tauri desktop shell",
    },
    { name: "TERMINAL", status: "LIMITED", note: "zylcode CLI (repo-context, build)" },
    { name: "ANDROID", status: "NOT INSTALLED", note: "Android execution not commissioned" },
    { name: "IOS", status: "NOT INSTALLED", note: "iOS execution not commissioned" },
    { name: "CONTAINER", status: "NOT INSTALLED", note: "Container runtime not commissioned" },
    { name: "REMOTE", status: "NOT INSTALLED", note: "Remote targets not commissioned" },
    {
      name: "LOGS",
      status: IS_DESKTOP ? "AVAILABLE" : "LIMITED",
      note: IS_DESKTOP ? "Live streams active" : "Streams require desktop runtime",
    },
  ];
  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-2 min-w-0">
      {targets.map((t) => (
        <Panel key={t.name}>
          <div className="flex flex-col gap-1.5 min-w-0">
            <div className="flex items-center justify-between gap-2">
              <span className="font-mono text-xs font-semibold shrink-0">{t.name}</span>
              <span
                className={`status-badge status-${t.status.toLowerCase().replace(" ", "-")} size-xs shrink-0`}
              >
                {t.status}
              </span>
            </div>
            <p className="text-[11px] text-text-muted break-words">{t.note}</p>
          </div>
        </Panel>
      ))}
    </div>
  );
}