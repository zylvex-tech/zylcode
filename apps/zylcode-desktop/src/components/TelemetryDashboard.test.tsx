import { render, screen, fireEvent, cleanup } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { TelemetryDashboard } from "./TelemetryDashboard";
import { BridgeMonitor, type BridgeServer } from "./BridgeMonitor";
import { ResourceGraph, type MetricPoint } from "./ResourceGraph";

// ---------------------------------------------------------------------------
// Telemetry surface (TelemetryDashboard + BridgeMonitor + ResourceGraph).
//
// These three are props-only views over real readings: a bridge inventory, a
// memory/token series, and the audit-log entry point. Nothing here fetches, so
// the assertions are about what each reading is reported *as* — the online
// count, the circuit state, the peak memory and the token-savings figure — plus
// the one interaction the dashboard owns: opening the audit logs.
// ---------------------------------------------------------------------------

const BRIDGES: BridgeServer[] = [
  {
    id: "b1",
    name: "filesystem",
    transport: "stdio",
    status: "connected",
    latencyMs: 12,
    circuitBreaker: "closed",
  },
  {
    id: "b2",
    name: "github",
    transport: "sse",
    status: "degraded",
    latencyMs: 240,
    circuitBreaker: "half-open",
  },
  {
    id: "b3",
    name: "postgres",
    transport: "stdio",
    status: "disconnected",
    latencyMs: 0,
    circuitBreaker: "open",
  },
];

const METRICS: MetricPoint[] = [
  { timestamp: "t1", memoryMb: 128, tokenSavingsRatio: 0.42 },
  { timestamp: "t2", memoryMb: 256, tokenSavingsRatio: 0.615 },
];

afterEach(() => cleanup());

describe("BridgeMonitor", () => {
  it("counts the online bridges out of the total", () => {
    render(<BridgeMonitor bridges={BRIDGES} />);

    expect(screen.getByText("1/3 Online")).toBeInTheDocument();
  });

  it("renders every bridge with its transport, latency and circuit state", () => {
    render(<BridgeMonitor bridges={BRIDGES} />);

    expect(screen.getByText("filesystem")).toBeInTheDocument();
    expect(screen.getAllByText("stdio")).toHaveLength(2);
    expect(screen.getByText("sse")).toBeInTheDocument();
    expect(screen.getByText("12ms")).toBeInTheDocument();
    expect(screen.getByText("[closed]")).toBeInTheDocument();

    expect(screen.getByText("github")).toBeInTheDocument();
    expect(screen.getByText("[half-open]")).toBeInTheDocument();
    expect(screen.getByText("[open]")).toBeInTheDocument();
  });

  it("reports an empty inventory as 0/0 online rather than hiding the panel", () => {
    render(<BridgeMonitor bridges={[]} />);

    expect(screen.getByText("0/0 Online")).toBeInTheDocument();
  });
});

describe("ResourceGraph", () => {
  it("reports the latest token-savings reading as a percentage", () => {
    render(<ResourceGraph data={METRICS} />);

    expect(screen.getByText("61.5% Token Savings")).toBeInTheDocument();
  });

  it("peaks the axis at the highest reading, not below it", () => {
    render(<ResourceGraph data={METRICS} />);

    expect(screen.getByText("Peak: 256.0 MB")).toBeInTheDocument();
  });

  it("falls back to a 64 MB floor when every reading is lower", () => {
    render(<ResourceGraph data={[{ timestamp: "t1", memoryMb: 8, tokenSavingsRatio: 0.1 }]} />);

    expect(screen.getByText("Peak: 64.0 MB")).toBeInTheDocument();
  });

  it("reports 0% savings and an empty polyline when there are no readings", () => {
    render(<ResourceGraph data={[]} />);

    expect(screen.getByText("0.0% Token Savings")).toBeInTheDocument();
    expect(screen.getByText("Peak: 64.0 MB")).toBeInTheDocument();
    expect(document.querySelector("polyline")).toHaveAttribute("points", "");
  });
});

describe("TelemetryDashboard", () => {
  it("shows both panels and hands the audit-log button to its caller", () => {
    const onOpenAuditLogs = vi.fn();
    render(
      <TelemetryDashboard bridges={BRIDGES} metrics={METRICS} onOpenAuditLogs={onOpenAuditLogs} />,
    );

    expect(screen.getByRole("heading", { name: "System Telemetry" })).toBeInTheDocument();
    expect(screen.getByText("1/3 Online")).toBeInTheDocument();
    expect(screen.getByText("61.5% Token Savings")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "View Audit Logs" }));
    expect(onOpenAuditLogs).toHaveBeenCalledTimes(1);
  });
});
