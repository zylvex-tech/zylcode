import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SettingsSurface } from "./SettingsSurface";
import { AUTONOMY_MODES } from "../lib/mission";
import { DEFAULT_SETTINGS } from "../lib/settings";
import { clearDiagnostics, recordDiagnostic } from "../lib/runtime";
import type { VersionState } from "../lib/version";

// ---------------------------------------------------------------------------
// Settings surface (components/SettingsSurface.tsx).
//
// The rules under test:
//   * closed renders nothing; open is a labelled dialog with eight sections;
//   * every control maps to a real persisted setting (mode, tab width,
//     toggles, autonomy) or to an honest statement (providers LIMITED in the
//     browser, hotkeys not rebindable yet);
//   * Escape / backdrop / ✕ / Done all close through the same callback;
//   * diagnostics and version rows read real runtime state.
// Heavy leaf panels are stubbed; each has its own test file.
// ---------------------------------------------------------------------------

vi.mock("./AgentTurnPanel", () => ({ default: () => <div data-testid="agent-turn-stub" /> }));
vi.mock("./ToolRunnerPanel", () => ({ default: () => <div data-testid="tool-runner-stub" /> }));
vi.mock("./ToolsCataloguePanel", () => ({
  default: () => <div data-testid="tools-stub" />,
}));
vi.mock("./McpInspector", () => ({ default: () => <div data-testid="mcp-stub" /> }));
vi.mock("./ProviderSettings", () => ({
  default: () => <div data-testid="provider-settings-stub" />,
}));
vi.mock("./ui", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./ui")>();
  return {
    ...actual,
    // ThemeSelector needs the app-level ThemeProvider, which is exercised by
    // its own surface; here the settings row wiring is what matters.
    ThemeSelector: () => <div data-testid="theme-selector-stub" />,
  };
});
vi.mock("../lib/version", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/version")>();
  return { ...actual, fetchVersion: vi.fn() };
});

const { fetchVersion } = await import("../lib/version");
const ver = vi.mocked(fetchVersion);

const versionReady: VersionState = {
  kind: "ready",
  data: {
    app_version: "0.2.1",
    engine: "0.9.0",
    head_commit: "abc1234",
    branch: "main",
    dirty: true,
  },
};

function open(initialSection?: "general" | "agent" | "providers" | "extensions" | "appearance" | "hotkeys" | "diagnostics" | "about") {
  const onClose = vi.fn();
  render(<SettingsSurface open initialSection={initialSection} onClose={onClose} />);
  return { onClose };
}

beforeEach(() => {
  localStorage.clear();
  clearDiagnostics();
  ver.mockReset().mockResolvedValue(versionReady);
});

afterEach(() => {
  vi.useRealTimers();
  vi.clearAllMocks();
});

describe("SettingsSurface", () => {
  it("renders nothing while closed", () => {
    const onClose = vi.fn();
    const { rerender } = render(<SettingsSurface open={false} onClose={onClose} />);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    rerender(<SettingsSurface open onClose={onClose} />);
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("offers all eight sections and shows the initial section's content", async () => {
    open();
    expect(screen.getByRole("dialog", { name: "Settings" })).toBeInTheDocument();
    for (const label of [
      "General",
      "Agent & Sessions",
      "Model Providers",
      "Extensions & MCP",
      "Appearance",
      "Hotkeys",
      "Diagnostics",
      "About",
    ]) {
      expect(screen.getByRole("button", { name: new RegExp(label) })).toBeInTheDocument();
    }
    expect(screen.getByText("Default mission mode")).toBeInTheDocument();
  });

  it("persists the general controls to real settings", () => {
    open();
    const plan = screen.getByRole("radio", { name: /Plan/ });
    fireEvent.click(plan);
    expect(plan).toHaveAttribute("aria-checked", "true");

    fireEvent.click(screen.getByRole("radio", { name: "4" }));
    expect(screen.getByRole("radio", { name: "4" })).toHaveAttribute("aria-checked", "true");

    const diag = screen.getByRole("switch", { name: "Capture diagnostics" });
    expect(diag).toHaveAttribute("aria-checked", String(DEFAULT_SETTINGS.captureDiagnostics));
    fireEvent.click(diag);
    expect(diag).toHaveAttribute(
      "aria-checked",
      String(!DEFAULT_SETTINGS.captureDiagnostics),
    );
  });

  it("resets every setting back to defaults", () => {
    open();
    fireEvent.click(screen.getByRole("radio", { name: /Plan/ }));
    expect(screen.getByRole("radio", { name: /Plan/ })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    fireEvent.click(screen.getByText("Reset all settings to defaults"));
    expect(screen.getByRole("radio", { name: /Build/ })).toHaveAttribute(
      "aria-checked",
      "true",
    );
  });

  it("closes from Escape, backdrop, ✕ and Done — all through onClose", () => {
    const onClose = vi.fn();
    render(<SettingsSurface open onClose={onClose} />);

    fireEvent.keyDown(window, { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);

    const backdrop = screen.getByRole("dialog").parentElement!;
    fireEvent.mouseDown(backdrop);
    expect(onClose).toHaveBeenCalledTimes(2);

    fireEvent.click(screen.getByRole("button", { name: "Close settings" }));
    expect(onClose).toHaveBeenCalledTimes(3);

    fireEvent.click(screen.getByRole("button", { name: "Done" }));
    expect(onClose).toHaveBeenCalledTimes(4);
  });

  it("lists every autonomy mode as an option in the Agent section", () => {
    open("agent");
    const select = screen.getByRole("combobox");
    expect(select).toBeInTheDocument();
    const options = within(select).getAllByRole("option");
    expect(options).toHaveLength(AUTONOMY_MODES.length);
    expect(screen.getByRole("switch", { name: "Tool call summary" })).toBeInTheDocument();
    expect(screen.getByRole("switch", { name: "Auto-fold finished work" })).toBeInTheDocument();
    expect(screen.getByTestId("agent-turn-stub")).toBeInTheDocument();
    expect(
      screen.getByText(/Queued missions never run without an explicit human action/),
    ).toBeInTheDocument();
  });

  it("says providers are LIMITED in the browser instead of faking an editor", () => {
    open("providers");
    expect(screen.getByText("LIMITED")).toBeInTheDocument();
    expect(
      screen.getByText(/cannot modify the engine&apos;s provider chain|cannot modify the engine's provider chain/),
    ).toBeInTheDocument();
    expect(screen.queryByTestId("provider-settings-stub")).not.toBeInTheDocument();
  });

  it("renders the extensions panels with the honest marketplace note", () => {
    open("extensions");
    expect(screen.getByTestId("tools-stub")).toBeInTheDocument();
    expect(screen.getByTestId("tool-runner-stub")).toBeInTheDocument();
    expect(
      screen.getByText(/marketplace install pipeline is not commissioned yet/),
    ).toBeInTheDocument();
  });

  it("appearance section drives the real theme selector and toggles", () => {
    open("appearance");
    expect(screen.getByText("Theme")).toBeInTheDocument();
    for (const label of [
      "Reduced motion",
      "Compact density",
      "Editor line numbers",
      "Editor word wrap",
    ]) {
      expect(screen.getByRole("switch", { name: label })).toBeInTheDocument();
    }
    const wrap = screen.getByRole("switch", { name: "Editor word wrap" });
    expect(wrap).toHaveAttribute("aria-checked", String(DEFAULT_SETTINGS.editorWordWrap));
    fireEvent.click(wrap);
    expect(wrap).toHaveAttribute("aria-checked", String(!DEFAULT_SETTINGS.editorWordWrap));
  });

  it("lists the built-in hotkeys and admits rebinding is not commissioned", () => {
    open("hotkeys");
    for (const key of [
      "Ctrl+Shift+P",
      "Ctrl+`",
      "Ctrl+Shift+E",
      "Ctrl+Shift+F",
      "Ctrl+Shift+G",
      "Ctrl+M",
      "Esc",
    ]) {
      expect(screen.getByText(key)).toBeInTheDocument();
    }
    expect(
      screen.getByText(/Custom hotkey rebinding is not commissioned yet/),
    ).toBeInTheDocument();
  });

  it("shows real engine version and live diagnostics count", async () => {
    recordDiagnostic("ipc", "invoke failed");
    vi.useFakeTimers();
    open("diagnostics");
    // Flush the version promise and the first interval pull.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(10);
    });

    expect(screen.getByText("0.2.1")).toBeInTheDocument();
    expect(
      screen.getByText("Engine 0.9.0 · main@abc1234 (dirty tree)"),
    ).toBeInTheDocument();
    expect(screen.getByText("1")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Clear" }));
    // The count refreshes on the panel's own 2s tick.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(2100);
    });
    expect(screen.getByText("0")).toBeInTheDocument();
  });

  it("falls back honestly when the engine version cannot be read", async () => {
    ver.mockResolvedValue({ kind: "unavailable", reason: "engine not reachable" });
    open("diagnostics");
    expect(await screen.findByText("engine not reachable")).toBeInTheDocument();
    expect(screen.getByText("—")).toBeInTheDocument();
  });

  it("shows the about panel with the honesty statement", async () => {
    open("about");
    expect(screen.getByAltText("ZylCode")).toBeInTheDocument();
    expect(
      screen.getByText("Formally Verified AI Software Synthesis Engine"),
    ).toBeInTheDocument();
    expect(screen.getByText("App version")).toBeInTheDocument();
    expect(
      screen.getByText(/When a capability is not commissioned, the UI says so/),
    ).toBeInTheDocument();
    expect(await screen.findByText("main@abc1234 · dirty")).toBeInTheDocument();
  });

  it("reports the settings schema footer state", () => {
    open();
    expect(screen.getByText("Settings persist locally")).toBeInTheDocument();
  });
});