// repoContext payload-mapping tests (P15 W4).
//
// Guards: a well-formed payload maps to its exact state with both absences
// preserved as nulls; an engine error maps to `unavailable` carrying the
// engine's own message; anything unrecognised (unknown state, missing
// fingerprint, non-object) maps to `unavailable` — a malformed payload is
// never guessed into a state.

import { describe, expect, it } from "vitest";

import { signalFromPayload } from "./repoContext";

describe("signalFromPayload", () => {
  it("maps a fresh payload verbatim", () => {
    expect(
      signalFromPayload({
        state: "fresh",
        fingerprint: "a1b2c3d4e5f6",
        indexed_fingerprint: "a1b2c3d4e5f6",
        changed_files: 0,
      }),
    ).toEqual({
      kind: "fresh",
      fingerprint: "a1b2c3d4e5f6",
      indexedFingerprint: "a1b2c3d4e5f6",
      changedFiles: 0,
    });
  });

  it("maps a stale payload with its change count", () => {
    expect(
      signalFromPayload({
        state: "stale",
        fingerprint: "ffff00001111",
        indexed_fingerprint: "a1b2c3d4e5f6",
        changed_files: 3,
      }),
    ).toEqual({
      kind: "stale",
      fingerprint: "ffff00001111",
      indexedFingerprint: "a1b2c3d4e5f6",
      changedFiles: 3,
    });
  });

  it("keeps an absent index as nulls, never as a fabricated baseline", () => {
    const signal = signalFromPayload({
      state: "stale",
      fingerprint: "ffff00001111",
      indexed_fingerprint: null,
      changed_files: null,
    });
    expect(signal).toEqual({
      kind: "stale",
      fingerprint: "ffff00001111",
      indexedFingerprint: null,
      changedFiles: null,
    });
  });

  it("maps reindexing", () => {
    expect(
      signalFromPayload({ state: "reindexing", fingerprint: "a1b2c3d4e5f6" })
        .kind,
    ).toBe("reindexing");
  });

  it("renders an engine error as unavailable with the engine's message", () => {
    const signal = signalFromPayload({
      error: "freshness probe failed: workspace root does not exist",
    });
    expect(signal).toEqual({
      kind: "unavailable",
      reason: "freshness probe failed: workspace root does not exist",
    });
  });

  it("never guesses an unknown state", () => {
    const signal = signalFromPayload({
      state: "very-fresh",
      fingerprint: "a1b2c3d4e5f6",
    });
    expect(signal.kind).toBe("unavailable");
    expect(signal.kind === "unavailable" && signal.reason).toContain(
      "very-fresh",
    );
  });

  it("rejects a payload with no fingerprint", () => {
    expect(signalFromPayload({ state: "fresh" }).kind).toBe("unavailable");
  });

  it("rejects a payload that is not an object", () => {
    expect(signalFromPayload(null).kind).toBe("unavailable");
    expect(signalFromPayload("fresh").kind).toBe("unavailable");
  });
});
