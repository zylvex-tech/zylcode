import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DeliveryPanel } from "./DeliveryPanel";
import type { BuildState, DeployState } from "../lib/delivery";

// ---------------------------------------------------------------------------
// Delivery surface (components/DeliveryPanel.tsx).
//
// The claims under test are all "do not over-claim":
//   * A pipeline that has never run shows COMING SOON, not AVAILABLE — the
//     pipeline existing is not the pipeline having run.
//   * A running pipeline disables the action buttons so a double click cannot
//     start a second run.
//   * A failed run shows red, with the engine's error verbatim.
//   * Launching a build surfaces the backend's own note or error rather than
//     an invented success message.
// ---------------------------------------------------------------------------

vi.mock("../lib/delivery", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/delivery")>();
  return {
    ...actual,
    fetchDeploy: vi.fn(),
    fetchBuildState: vi.fn(),
    startBuild: vi.fn(),
    startPackage: vi.fn(),
  };
});

const { fetchDeploy, fetchBuildState, startBuild, startPackage } = await import("../lib/delivery");

const DEPLOY: DeployState = {
  kind: "ready",
  head: "43f11ed",
  github_release: { commissioned: true, state: "LIVE", note: "v0.2.2 published" },
  crates_io: { commissioned: false, state: "UNCOMMISSIONED", note: "" },
  remote_server: { commissioned: false, state: "UNCOMMISSIONED", note: "" },
};

const IDLE: BuildState = { kind: "ready", state: "idle", steps: [] };

beforeEach(() => {
  vi.mocked(fetchDeploy).mockReset().mockResolvedValue(DEPLOY);
  vi.mocked(fetchBuildState).mockReset().mockResolvedValue(IDLE);
  vi.mocked(startBuild).mockReset();
  vi.mocked(startPackage).mockReset();
});

afterEach(() => {
  vi.clearAllMocks();
});

/**
 * A successful launch schedules a 500 ms status refresh. If a test ends
 * before it fires, the callback runs against reset mocks and throws as an
 * unhandled error *after* the suite — which vitest reports as "tests pass,
 * run failed". Waiting it out turns the hazard into an assertion: the panel
 * really does re-read the pipeline after launching one.
 */
async function flushDeferredRefresh() {
  const before = vi.mocked(fetchBuildState).mock.calls.length;
  await new Promise((resolve) => setTimeout(resolve, 600));
  expect(vi.mocked(fetchBuildState).mock.calls.length).toBeGreaterThan(before);
}

describe("DeliveryPanel", () => {
  it("shows a loading state before the first response", () => {
    vi.mocked(fetchDeploy).mockReturnValue(new Promise(() => {}));
    vi.mocked(fetchBuildState).mockReturnValue(new Promise(() => {}));

    render(<DeliveryPanel />);

    expect(screen.getByText("loading build status…")).toBeInTheDocument();
  });

  it("marks an idle pipeline COMING SOON rather than AVAILABLE", async () => {
    render(<DeliveryPanel />);

    await waitFor(() => expect(screen.getByText("COMING SOON")).toBeInTheDocument());
    expect(screen.getByText("no pipeline run in this service session")).toBeInTheDocument();
  });

  it("renders each build step with its outcome and duration", async () => {
    vi.mocked(fetchBuildState).mockResolvedValue({
      kind: "ready",
      state: "passed",
      steps: [
        {
          name: "cargo test",
          command: "cargo test",
          passed: true,
          exit_code: 0,
          duration_ms: 1200,
        },
        {
          name: "cargo clippy",
          command: "cargo clippy",
          passed: false,
          exit_code: 101,
          duration_ms: 400,
        },
      ],
    });

    render(<DeliveryPanel />);

    await waitFor(() => expect(screen.getByText("cargo test")).toBeInTheDocument());
    expect(screen.getByText("cargo clippy")).toBeInTheDocument();
    expect(screen.getByText("1.2s")).toBeInTheDocument();
    expect(screen.getByText("0.4s")).toBeInTheDocument();
    // The build section's badge — the deploy section is AVAILABLE too.
    const available = screen.getAllByText("AVAILABLE");
    expect(available.length).toBeGreaterThanOrEqual(1);
  });

  it("shows a failed run red with the engine's error verbatim", async () => {
    vi.mocked(fetchBuildState).mockResolvedValue({
      kind: "ready",
      state: "failed",
      error: "step `cargo test` exited 101",
      steps: [],
    });

    render(<DeliveryPanel />);

    await waitFor(() => expect(screen.getByText("BLOCKED")).toBeInTheDocument());
    expect(screen.getByText(/step `cargo test` exited 101/)).toBeInTheDocument();
  });

  it("disables the run and package actions while a pipeline is running", async () => {
    vi.mocked(fetchBuildState).mockResolvedValue({
      kind: "ready",
      state: "running",
      steps: [],
    });

    render(<DeliveryPanel />);

    await waitFor(() => expect(screen.getByText("LIMITED")).toBeInTheDocument());
    expect(screen.getByRole("button", { name: "Run build pipeline" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Package release" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Package only" })).toBeDisabled();
  });

  it("surfaces a service failure as an unavailable build status", async () => {
    vi.mocked(fetchBuildState).mockResolvedValue({
      kind: "unavailable",
      reason: "serve-intel is not running",
    });

    render(<DeliveryPanel />);

    await waitFor(() =>
      expect(
        screen.getByText("build status unavailable: serve-intel is not running"),
      ).toBeInTheDocument(),
    );
  });

  it("shows commission state per target and claims no deploy that did not happen", async () => {
    render(<DeliveryPanel />);

    await waitFor(() => expect(screen.getByText("43f11ed")).toBeInTheDocument());
    expect(screen.getByText("GitHub Releases")).toBeInTheDocument();
    expect(screen.getByText("crates.io")).toBeInTheDocument();
    // crates.io and the remote server are both uncommissioned.
    expect(screen.getAllByText("UNCOMMISSIONED")).toHaveLength(2);
    expect(
      screen.getByText(/nothing here claims a deploy that did not happen/),
    ).toBeInTheDocument();
  });

  it("reports the backend's own note when a build is launched", async () => {
    vi.mocked(startBuild).mockResolvedValue({ started: true, note: "pipeline queued" });
    const user = userEvent.setup();

    render(<DeliveryPanel />);
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Run build pipeline" })).toBeEnabled(),
    );

    await user.click(screen.getByRole("button", { name: "Run build pipeline" }));

    await waitFor(() => expect(screen.getByText("pipeline queued")).toBeInTheDocument());
    expect(startBuild).toHaveBeenCalledTimes(1);
    await flushDeferredRefresh();
  });

  it("reports the backend's own error when the launch is refused", async () => {
    vi.mocked(startBuild).mockResolvedValue({
      started: false,
      error: "a pipeline is already running",
    });
    const user = userEvent.setup();

    render(<DeliveryPanel />);
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Run build pipeline" })).toBeEnabled(),
    );

    await user.click(screen.getByRole("button", { name: "Run build pipeline" }));

    await waitFor(() =>
      expect(screen.getByText("a pipeline is already running")).toBeInTheDocument(),
    );
  });

  it("sends the typed version when packaging a release", async () => {
    vi.mocked(startPackage).mockResolvedValue({ started: true, note: "packaging 0.3.0" });
    const user = userEvent.setup();

    render(<DeliveryPanel />);
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Package release" })).toBeEnabled(),
    );

    await user.type(screen.getByPlaceholderText("version e.g. 0.3.0"), "0.3.0");
    await user.click(screen.getByRole("button", { name: "Package release" }));

    await waitFor(() => expect(startPackage).toHaveBeenCalledWith("0.3.0", false));
    await flushDeferredRefresh();
  });

  it("packaging without a build passes packageOnly=true", async () => {
    vi.mocked(startPackage).mockResolvedValue({ started: true });
    const user = userEvent.setup();

    render(<DeliveryPanel />);
    await waitFor(() => expect(screen.getByRole("button", { name: "Package only" })).toBeEnabled());

    await user.click(screen.getByRole("button", { name: "Package only" }));

    await waitFor(() => expect(startPackage).toHaveBeenCalledWith("", true));
    await flushDeferredRefresh();
  });
});
