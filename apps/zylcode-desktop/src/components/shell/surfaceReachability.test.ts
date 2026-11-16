import { describe, expect, it } from "vitest";
import { ALL_ACTIVITY_IDS } from "./ActivityRail";
import { ACTIVITY_TO_SURFACE } from "./navigation";
import { SURFACES, type SurfaceType } from "./SurfaceHost";

// ---------------------------------------------------------------------------
// Surface reachability (components/shell/SurfaceHost.tsx).
//
// Rendering a surface is not the same as shipping it. The Design workspace was
// implemented, tested and unreachable: `SurfaceHost` had a `design` case, but
// ActivityId had no `design`, the rail had no button and the mapping had no
// entry, so nothing a user could click ever got there.
//
// This test makes that class of defect loud. A surface must have an entry point:
// either an activity-rail destination, or a documented route opened some other
// way, or an entry in the declared-debt list below — which the second test
// forces to shrink the moment one of those surfaces becomes reachable.
// ---------------------------------------------------------------------------

/** Surfaces with a real entry point that is not an activity-rail button. */
const REACHED_ELSEWHERE: Partial<Record<SurfaceType, string>> = {
  home: "the app opens here, and the ZylCode wordmark navigates back (App.tsx)",
  overview: "the Settings and Account utilities both map here",
};

/**
 * Declared debt: rendered, but nothing navigates to it yet.
 *
 * Not an exemption — a recorded gap. The second test fails if one of these
 * becomes reachable and is left here, so this list can only shrink by being
 * corrected.
 */
const KNOWN_UNREACHABLE: Partial<Record<SurfaceType, string>> = {
  artifacts: "SurfaceHost renders it; no control navigates to it yet",
  evidence: "SurfaceHost renders it; the rail's Evidence activity opens evidence-live instead",
};

function railSurfaces(): Set<string> {
  return new Set<string>(Object.values(ACTIVITY_TO_SURFACE));
}

describe("surface reachability", () => {
  it("gives every declared surface an entry point", () => {
    const reachable = railSurfaces();
    const undocumented = SURFACES.filter(
      (surface) =>
        !reachable.has(surface) &&
        !(surface in REACHED_ELSEWHERE) &&
        !(surface in KNOWN_UNREACHABLE),
    );

    expect(
      undocumented,
      `these surfaces render but nothing can navigate to them: ${undocumented.join(", ")}. ` +
        "Add a rail activity that maps to each one, or record how it is reached another way.",
    ).toEqual([]);
  });

  it("does not let the declared-debt list hide a reachable or invented surface", () => {
    const reachable = railSurfaces();

    for (const [surface, reason] of Object.entries(KNOWN_UNREACHABLE)) {
      expect(SURFACES, `${surface} is not a declared surface`).toContain(surface as SurfaceType);
      expect(
        reachable.has(surface),
        `${surface} is reachable now — remove it from KNOWN_UNREACHABLE`,
      ).toBe(false);
      expect(reason.length).toBeGreaterThan(10);
    }

    for (const [surface, reason] of Object.entries(REACHED_ELSEWHERE)) {
      expect(SURFACES, `${surface} is not a declared surface`).toContain(surface as SurfaceType);
      expect(reason.length).toBeGreaterThan(10);
    }
  });

  it("maps every rail activity to a surface that exists", () => {
    expect(ALL_ACTIVITY_IDS.length).toBeGreaterThan(0);

    for (const id of ALL_ACTIVITY_IDS) {
      expect(Object.keys(ACTIVITY_TO_SURFACE), `${id} has no surface mapping`).toContain(id);
      expect(SURFACES).toContain(ACTIVITY_TO_SURFACE[id]);
    }
  });

  it("has no mapping for an activity the rail no longer offers", () => {
    expect(Object.keys(ACTIVITY_TO_SURFACE).sort()).toEqual([...ALL_ACTIVITY_IDS].sort());
  });
});
