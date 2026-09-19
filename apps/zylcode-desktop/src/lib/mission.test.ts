import { describe, expect, it } from "vitest";
import {
  AUTONOMY_MODES,
  EVIDENCE_CATEGORIES,
  NOT_CAPTURED,
  newMissionId,
} from "./mission";

// ---------------------------------------------------------------------------
// Mission domain (v3 §5–§7): states, autonomy modes with truthful
// availability, evidence categories with the NOT CAPTURED contract.
// ---------------------------------------------------------------------------

describe("mission states", () => {
  it("newMissionId produces unique prefixed ids", () => {
    const a = newMissionId();
    const b = newMissionId();
    expect(a).toMatch(/^mission-/);
    expect(a).not.toBe(b);
  });
});

describe("AUTONOMY_MODES", () => {
  it("exposes all five designed modes", () => {
    const modes = AUTONOMY_MODES.map((m) => m.mode);
    expect(modes).toEqual(["OBSERVE", "GUIDED", "BUILD", "MISSION", "SOVEREIGN"]);
  });

  it("does not claim every mode is fully available (honesty contract)", () => {
    const byMode = new Map(AUTONOMY_MODES.map((m) => [m.mode, m.status]));
    expect(byMode.get("SOVEREIGN")).toBe("COMING SOON");
    expect(byMode.get("OBSERVE")).toBe("AVAILABLE");
    expect(byMode.get("GUIDED")).toBe("AVAILABLE");
    // BUILD/MISSION are wired but bounded — they must not claim AVAILABLE.
    expect(byMode.get("BUILD")).not.toBe("AVAILABLE");
    expect(byMode.get("MISSION")).not.toBe("AVAILABLE");
  });

  it("keeps every status inside the universal status language", () => {
    const allowed = ["AVAILABLE", "LIMITED", "COMING SOON", "BLOCKED", "NOT INSTALLED"];
    for (const m of AUTONOMY_MODES) {
      expect(allowed).toContain(m.status);
    }
  });
});

describe("evidence model", () => {
  it("defines the nine evidence categories (v3 §12)", () => {
    expect(EVIDENCE_CATEGORIES).toEqual([
      "Build",
      "Tests",
      "Runtime",
      "Screenshots",
      "Tool actions",
      "Approvals",
      "Git changes",
      "Artifacts",
      "Verification",
    ]);
  });

  it("uses NOT CAPTURED as the universal honest answer for absent evidence", () => {
    expect(NOT_CAPTURED).toBe("NOT CAPTURED");
  });
});
