import { describe, expect, it, beforeEach, vi } from "vitest";
import { act, renderHook } from "@testing-library/react";
import {
  DEFAULT_SETTINGS,
  loadSettings,
  mergeSettings,
  saveSettings,
  useSettings,
} from "./settings";

describe("settings store", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it("returns defaults when nothing is persisted", () => {
    expect(loadSettings()).toEqual(DEFAULT_SETTINGS);
  });

  it("persists and reloads a valid update", () => {
    const next = { ...DEFAULT_SETTINGS, defaultMissionMode: "plan" as const };
    saveSettings(next);
    expect(loadSettings().defaultMissionMode).toBe("plan");
  });

  it("drops fields with wrong types instead of crashing", () => {
    const merged = mergeSettings({
      editorWordWrap: "yes",
      editorTabSize: 7,
      defaultMissionMode: "deploy",
      defaultAutonomy: "GUIDED",
    });
    expect(merged.editorWordWrap).toBe(DEFAULT_SETTINGS.editorWordWrap);
    expect(merged.editorTabSize).toBe(DEFAULT_SETTINGS.editorTabSize);
    expect(merged.defaultMissionMode).toBe(DEFAULT_SETTINGS.defaultMissionMode);
    // valid fields still applied
    expect(merged.defaultAutonomy).toBe("GUIDED");
  });

  it("drops out-of-set enum values", () => {
    const merged = mergeSettings({ defaultAutonomy: "CHAOS" });
    expect(merged.defaultAutonomy).toBe(DEFAULT_SETTINGS.defaultAutonomy);
  });

  it("falls back to defaults for corrupt JSON", () => {
    window.localStorage.setItem("zylcode.settings.v1", "{not json");
    expect(loadSettings()).toEqual(DEFAULT_SETTINGS);
  });

  it("returns defaults for null or non-object input rather than throwing", () => {
    expect(mergeSettings(null)).toEqual(DEFAULT_SETTINGS);
    expect(mergeSettings(undefined)).toEqual(DEFAULT_SETTINGS);
    expect(mergeSettings("not an object")).toEqual(DEFAULT_SETTINGS);
    expect(mergeSettings(42)).toEqual(DEFAULT_SETTINGS);
  });

  it("ignores explicit nulls as well as missing fields", () => {
    const merged = mergeSettings({ editorWordWrap: null, toolCallSummary: null });

    expect(merged.editorWordWrap).toBe(DEFAULT_SETTINGS.editorWordWrap);
    expect(merged.toolCallSummary).toBe(DEFAULT_SETTINGS.toolCallSummary);
  });

  it("keeps runtime state when persistence is denied (private mode)", () => {
    // Documented as best-effort: a browser that refuses storage must not
    // throw out of saveSettings — the in-memory settings still apply.
    const spy = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("QuotaExceededError");
    });

    expect(() => saveSettings({ ...DEFAULT_SETTINGS, editorTabSize: 2 })).not.toThrow();
    expect(loadSettings()).toEqual(DEFAULT_SETTINGS);

    spy.mockRestore();
  });
});

describe("useSettings hook", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it("starts from what is already persisted", () => {
    saveSettings({ ...DEFAULT_SETTINGS, editorWordWrap: true });

    const { result } = renderHook(() => useSettings());

    expect(result.current.settings.editorWordWrap).toBe(true);
  });

  it("applies an update and persists it for the next launch", () => {
    const { result } = renderHook(() => useSettings());

    act(() => result.current.update("editorTabSize", 2));

    expect(result.current.settings.editorTabSize).toBe(2);
    expect(loadSettings().editorTabSize).toBe(2);
  });

  it("resets back to the shipped defaults", () => {
    const { result } = renderHook(() => useSettings());

    act(() => result.current.update("reducedMotion", true));
    act(() => result.current.reset());

    expect(result.current.settings).toEqual(DEFAULT_SETTINGS);
    expect(loadSettings()).toEqual(DEFAULT_SETTINGS);
  });

  it("re-reads settings when another tab writes them", () => {
    const { result } = renderHook(() => useSettings());

    act(() => {
      window.localStorage.setItem(
        "zylcode.settings.v1",
        JSON.stringify({ ...DEFAULT_SETTINGS, compactDensity: true }),
      );
      window.dispatchEvent(new StorageEvent("storage", { key: "zylcode.settings.v1" }));
    });

    expect(result.current.settings.compactDensity).toBe(true);
  });
});
