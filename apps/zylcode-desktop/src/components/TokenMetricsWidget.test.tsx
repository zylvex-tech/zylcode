import { render, screen, cleanup } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import TokenMetricsWidget from "./TokenMetricsWidget";
import { fetchMetrics, type MetricsState } from "../lib/service";

// ---------------------------------------------------------------------------
// Token metrics widget (components/TokenMetricsWidget.tsx).
//
// The widget renders *measured* telemetry through the browser transport, so
// every state of `fetchMetrics` is exercised here rather than faked:
//
//   * before the first reading arrives it says it is still reading;
//   * an unavailable metrics route states the backend's reason, never zeros;
//   * a real reading reports tokens, cache hit/miss, the cost estimate and
//     the saved-cost figure — with the sub-cent branch spelled "<$0.01".
// ---------------------------------------------------------------------------

vi.mock("../lib/service", () => ({ fetchMetrics: vi.fn() }));

const metrics = vi.mocked(fetchMetrics);

const READY: MetricsState = {
  kind: "ready",
  input_tokens: 2_000_000,
  output_tokens: 500_000,
  verification_saved_tokens: 100_000,
  fallback_count: 2,
};

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

beforeEach(() => {
  metrics.mockReset();
});

describe("TokenMetricsWidget", () => {
  it("says it is still reading before the first measurement lands", () => {
    metrics.mockReturnValue(new Promise<MetricsState>(() => {}));

    render(<TokenMetricsWidget />);

    expect(screen.getByText("metrics…")).toBeInTheDocument();
  });

  it("renders the measurement: tokens, cache hit, cost, savings and fallbacks", async () => {
    metrics.mockResolvedValue(READY);

    render(<TokenMetricsWidget />);

    expect(await screen.findByText(/tok 2000000\/500000/)).toBeInTheDocument();
    expect(await screen.findByText(/cache HIT/)).toBeInTheDocument();
    expect(await screen.findByText("cost $13.50")).toBeInTheDocument();
    expect(await screen.findByText("saved $0.30")).toBeInTheDocument();
    expect(await screen.findByText("fallback ×2")).toBeInTheDocument();
  });

  it("marks a miss and omits the savings when nothing was saved", async () => {
    metrics.mockResolvedValue({
      kind: "ready",
      input_tokens: 100,
      output_tokens: 50,
      verification_saved_tokens: 0,
      fallback_count: 0,
    });

    render(<TokenMetricsWidget />);

    expect(await screen.findByText(/cache —/)).toBeInTheDocument();
    expect(await screen.findByText("cost <$0.01")).toBeInTheDocument();
    // The cache row always ends in "saved"; only the cost row carries a
    // "saved $…" figure, and that branch must not render here.
    expect(screen.queryByText(/^saved/)).not.toBeInTheDocument();
    expect(screen.queryByText(/fallback/)).not.toBeInTheDocument();
  });

  it("states the backend's reason when metrics are unavailable, never zeros", async () => {
    metrics.mockResolvedValue({ kind: "unavailable", reason: "no scorecard route" });

    render(<TokenMetricsWidget />);

    const label = await screen.findByText("metrics: n/a");
    expect(label).toHaveAttribute("title", "no scorecard route");
  });

  it("re-reads the route when the caller bumps the refresh key", async () => {
    metrics.mockResolvedValue(READY);
    const view = render(<TokenMetricsWidget refreshKey={0} />);
    await screen.findByText(/tok 2000000/);

    view.rerender(<TokenMetricsWidget refreshKey={1} />);

    expect(metrics).toHaveBeenCalledTimes(2);
  });
});
