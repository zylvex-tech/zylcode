import { describe, expect, it, beforeEach } from "vitest";
import {
  DEFAULT_SETTINGS,
  loadSettings,
  mergeSettings,
  saveSettings,
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
});
