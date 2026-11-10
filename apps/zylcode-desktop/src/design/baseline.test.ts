import { beforeEach, describe, expect, it } from "vitest";
import {
  baselineKey,
  clearBaseline,
  diffSnapshots,
  readBaseline,
  saveBaseline,
  toBaseline,
} from "./baseline";

// ---------------------------------------------------------------------------
// The comparison foundation is property-based, so its diff has to be exact:
// a value that did not change is reported unchanged, and a missing value is a
// change rather than a crash.
// ---------------------------------------------------------------------------

const snapshot = {
  color: "rgb(248, 250, 252)",
  backgroundColor: "rgb(59, 130, 246)",
  fontSize: "14px",
};

describe("baseline comparison", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("captures only the values the browser reported", () => {
    const baseline = toBaseline(snapshot, "midnight-pro", 1024, ["--color-primary"]);
    expect(baseline.values).toEqual(snapshot);
    expect(baseline.theme).toBe("midnight-pro");
    expect(baseline.tokens).toEqual(["--color-primary"]);
    expect(Number.isNaN(Date.parse(baseline.capturedAt))).toBe(false);
  });

  it("reports unchanged, changed and newly empty properties", () => {
    const baseline = toBaseline(snapshot, "midnight-pro", 1024, []);
    const diffs = diffSnapshots(baseline, { ...snapshot, backgroundColor: "rgb(37, 99, 235)" });

    expect(diffs.find((d) => d.property === "color")?.changed).toBe(false);
    expect(diffs.find((d) => d.property === "backgroundColor")).toMatchObject({
      changed: true,
      baseline: "rgb(59, 130, 246)",
      current: "rgb(37, 99, 235)",
    });

    const dropped = diffSnapshots(baseline, { color: snapshot.color });
    expect(dropped.find((d) => d.property === "fontSize")).toMatchObject({
      changed: true,
      current: "",
    });
  });

  it("round-trips through storage and clears", () => {
    const key = baselineKey("ui/index#Button", "button", "btn btn-primary");
    expect(readBaseline(key)).toBeNull();

    const baseline = toBaseline(snapshot, "midnight-pro", 1440, []);
    expect(saveBaseline(key, baseline)).toBe(true);
    expect(readBaseline(key)?.theme).toBe("midnight-pro");

    clearBaseline(key);
    expect(readBaseline(key)).toBeNull();
  });

  it("returns null for corrupt storage rather than throwing", () => {
    const key = baselineKey("x", "div", "");
    localStorage.setItem(key, "{not json");
    expect(readBaseline(key)).toBeNull();
  });
});
