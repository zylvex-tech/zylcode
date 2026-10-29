import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import ProviderSettings from "./ProviderSettings";

// ---------------------------------------------------------------------------
// Provider settings (components/ProviderSettings.tsx).
//
// The rules under test:
//   * the fallback chain is read from the engine and rendered sorted, with
//     honest API-key / disabled badges — no secrets ever in the UI;
//   * Save validates the timeout range before touching the engine and sends
//     trimmed values (empty → null, i.e. "reset to default");
//   * enable/disable and reorder go through the engine, and an out-of-range
//     reorder is a no-op rather than an error;
//   * failover events stream into the live indicator list, capped at 20;
//   * engine failures surface verbatim instead of silently disappearing.
// ---------------------------------------------------------------------------

vi.mock("../lib/runtime", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/runtime")>();
  return { ...actual, safeInvoke: vi.fn() };
});
vi.mock("../lib/events", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/events")>();
  return { ...actual, safeListen: vi.fn() };
});

const { safeInvoke } = await import("../lib/runtime");
const { safeListen } = await import("../lib/events");
const invoke = vi.mocked(safeInvoke);
const listen = vi.mocked(safeListen);

type Config = {
  kind: "anthropic" | "open_router" | "ollama" | "synthetic_offline";
  model: string;
  endpoint: string;
  timeout_ms: number;
  enabled: boolean;
  fallback_order: number;
  requires_api_key: boolean;
};

const baseConfigs = (): Config[] => [
  {
    kind: "ollama",
    model: "llama3.1",
    endpoint: "",
    timeout_ms: 30000,
    enabled: false,
    fallback_order: 1,
    requires_api_key: false,
  },
  {
    kind: "anthropic",
    model: "claude-sonnet-4",
    endpoint: "https://api.anthropic.com",
    timeout_ms: 60000,
    enabled: true,
    fallback_order: 0,
    requires_api_key: true,
  },
];

let unlisten = vi.fn();
let failoverHandler: ((p: unknown) => void) | null = null;

/** The card containing the given provider label (or element). */
function cardOf(label: string | HTMLElement): HTMLElement {
  const node = typeof label === "string" ? screen.getByText(label) : label;
  const el = node.closest('div[class*="rounded border border-zyl-border"]');
  if (!el) throw new Error("no provider card found");
  return el as HTMLElement;
}

/** The input inside a labelled field of a card. */
function field(card: HTMLElement, label: string): HTMLInputElement {
  return within(card).getByText(label).closest("label")!.querySelector("input")!;
}

beforeEach(() => {
  unlisten = vi.fn();
  failoverHandler = null;
  invoke.mockReset().mockImplementation(async (_env, cmd, args) => {
    if (cmd === "get_provider_configs") return { ok: true, data: baseConfigs() } as never;
    if (cmd === "set_provider_config") {
      const patch = args as Record<string, unknown>;
      const next = baseConfigs().map((c) =>
        c.kind === patch.kind
          ? {
              ...c,
              ...(patch.endpoint !== null && patch.endpoint !== undefined
                ? { endpoint: String(patch.endpoint) }
                : {}),
              ...(patch.timeoutMs !== null && patch.timeoutMs !== undefined
                ? { timeout_ms: Number(patch.timeoutMs) }
                : {}),
              ...(patch.enabled !== null && patch.enabled !== undefined
                ? { enabled: Boolean(patch.enabled) }
                : {}),
              ...(patch.model !== null && patch.model !== undefined
                ? { model: String(patch.model) }
                : {}),
            }
          : c,
      );
      return { ok: true, data: next } as never;
    }
    if (cmd === "reorder_provider_chain") {
      const order = (args as { order: string[] }).order;
      return {
        ok: true,
        data: baseConfigs().map((c) => ({
          ...c,
          fallback_order: order.indexOf(c.kind),
        })),
      } as never;
    }
    return { ok: false, reason: "unknown command" } as never;
  });
  listen.mockReset().mockImplementation(async (_event, handler) => {
    failoverHandler = handler as (p: unknown) => void;
    return unlisten;
  });
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("ProviderSettings", () => {
  it("renders the chain sorted by fallback order with honest badges", async () => {
    render(<ProviderSettings />);
    expect(await screen.findByText("Provider Settings — Fallback Chain")).toBeInTheDocument();

    // The chain renders after the config load resolves — wait for it first.
    expect(await screen.findByText("Anthropic Native")).toBeInTheDocument();
    // Sorted: anthropic (#0) before ollama (#1) even though the payload wasn't.
    const cards = document.querySelectorAll('[class*="rounded border"]');
    const text = Array.from(cards).map((c) => c.textContent ?? "");
    const anthropicCard = text.findIndex((t) => t.includes("Anthropic Native"));
    const ollamaCard = text.findIndex((t) => t.includes("Ollama Local"));
    expect(anthropicCard).toBeGreaterThanOrEqual(0);
    expect(ollamaCard).toBeGreaterThan(anthropicCard);

    expect(screen.getByText("Anthropic Native")).toBeInTheDocument();
    expect(screen.getByText("#0 · anthropic")).toBeInTheDocument();
    expect(screen.getByText("API key")).toBeInTheDocument();
    expect(screen.getByText("Ollama Local")).toBeInTheDocument();
    expect(screen.getByText("disabled")).toBeInTheDocument();
    // Inputs are prefilled from the engine's own values.
    const anth = cardOf("Anthropic Native");
    expect(field(anth, "Model")).toHaveValue("claude-sonnet-4");
    expect(field(anth, "Endpoint override")).toHaveValue("https://api.anthropic.com");
    expect(field(anth, "Timeout ms")).toHaveValue("60000");
    const oll = cardOf("Ollama Local");
    expect(field(oll, "Endpoint override")).toHaveValue("");
    expect(field(oll, "Endpoint override")).toHaveAttribute(
      "placeholder",
      "http://localhost:11434",
    );
    expect(screen.getByText(/Order defines fallback priority/)).toBeInTheDocument();
  });

  it("re-reads the chain when Refresh is clicked", async () => {
    render(<ProviderSettings />);
    await screen.findByText("Anthropic Native");
    fireEvent.click(screen.getByRole("button", { name: "Refresh" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
    expect(invoke).toHaveBeenLastCalledWith(expect.anything(), "get_provider_configs");
  });

  it("Save validates the timeout range before calling the engine", async () => {
    render(<ProviderSettings />);
    const anth = cardOf(await screen.findByText("Anthropic Native"));

    fireEvent.change(field(anth, "Timeout ms"), { target: { value: "500" } });
    fireEvent.click(within(anth).getByRole("button", { name: "Save" }));

    expect(await screen.findByText(/timeout_ms must be 1000\.\.300000/)).toBeInTheDocument();
    // Validation happens before any engine call: only the initial load hit.
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  it("Save sends trimmed values, nulls for empties, and refreshes the chain", async () => {
    render(<ProviderSettings />);
    const anth = cardOf(await screen.findByText("Anthropic Native"));

    fireEvent.change(field(anth, "Model"), { target: { value: "  claude-3-opus  " } });
    fireEvent.change(field(anth, "Endpoint override"), { target: { value: "   " } });
    fireEvent.change(field(anth, "Timeout ms"), { target: { value: "120000" } });
    fireEvent.click(within(anth).getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(expect.anything(), "set_provider_config", {
        kind: "anthropic",
        endpoint: null, // empty → engine default
        timeoutMs: 120000,
        enabled: null,
        model: "claude-3-opus",
      }),
    );
    expect(screen.queryByText(/timeout_ms must be/)).not.toBeInTheDocument();
  });

  it("surfaces engine failures verbatim", async () => {
    invoke.mockImplementation(async (_env, cmd) => {
      if (cmd === "get_provider_configs") return { ok: true, data: baseConfigs() } as never;
      return { ok: false, reason: "engine rejected the config" } as never;
    });
    render(<ProviderSettings />);
    await screen.findByText("Anthropic Native");

    fireEvent.click(screen.getAllByRole("button", { name: "Save" })[0]);
    expect(await screen.findByText(/Error: engine rejected the config/)).toBeInTheDocument();
  });

  it("shows the initial load failure instead of an empty shell", async () => {
    invoke.mockResolvedValue({ ok: false, reason: "desktop engine not running" } as never);
    render(<ProviderSettings />);
    expect(await screen.findByText(/desktop engine not running/)).toBeInTheDocument();
    expect(screen.queryByText("Anthropic Native")).not.toBeInTheDocument();
  });

  it("toggles a provider through the engine", async () => {
    render(<ProviderSettings />);
    const anth = cardOf(await screen.findByText("Anthropic Native"));

    const checkbox = within(anth).getByRole("checkbox");
    expect(checkbox).toBeChecked();
    fireEvent.click(checkbox);

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(expect.anything(), "set_provider_config", {
        kind: "anthropic",
        endpoint: null,
        timeoutMs: null,
        enabled: false,
        model: null,
      }),
    );
  });

  it("reorders with the arrow buttons and refuses an out-of-range move", async () => {
    render(<ProviderSettings />);
    await screen.findByText("Anthropic Native");

    const up = screen.getAllByTitle("Move up (higher priority)")[0];
    fireEvent.click(up);
    // Already first: no engine call.
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(1));

    const down = screen.getAllByTitle("Move down (lower priority)")[0];
    fireEvent.click(down);
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(expect.anything(), "reorder_provider_chain", {
        order: ["ollama", "anthropic"],
      }),
    );
    // The engine's answer re-sorts the list: ollama is now #0.
    expect(await screen.findByText("#0 · ollama")).toBeInTheDocument();
  });

  it("disables the card controls while an engine call is in flight", async () => {
    let resolveSet: (v: unknown) => void = () => {};
    invoke.mockImplementation(async (_env, cmd) => {
      if (cmd === "get_provider_configs") return { ok: true, data: baseConfigs() } as never;
      return new Promise((r) => {
        resolveSet = r;
      }) as never;
    });
    render(<ProviderSettings />);
    await screen.findByText("Anthropic Native");

    const save = screen.getAllByRole("button", { name: "Save" })[0];
    fireEvent.click(save);
    expect(await screen.findByRole("button", { name: "Saving…" })).toBeDisabled();
    // Reorder arrows are locked while busy.
    expect(screen.getAllByTitle("Move up (higher priority)")[0]).toBeDisabled();

    await act(async () => {
      resolveSet({ ok: true, data: baseConfigs() });
    });
    await waitFor(() =>
      expect(screen.queryByRole("button", { name: "Saving…" })).not.toBeInTheDocument(),
    );
  });

  it("streams failover events into the live indicator list", async () => {
    render(<ProviderSettings />);
    await waitFor(() => expect(listen).toHaveBeenCalledTimes(1));
    expect(listen).toHaveBeenCalledWith("telemetry:provider_failover", expect.any(Function));
    expect(screen.getByText(/No failovers yet/)).toBeInTheDocument();

    act(() => {
      failoverHandler!({
        timestamp: "2026-10-28T10:00:00Z",
        from_provider: "anthropic",
        to_provider: "open_router",
        reason: "429 rate limited",
        attempt_number: 1,
      });
    });

    expect(screen.getByText("↻ #1")).toBeInTheDocument();
    expect(screen.getByText("anthropic")).toBeInTheDocument();
    expect(screen.getByText("open_router")).toBeInTheDocument();
    expect(screen.getByText("429 rate limited")).toBeInTheDocument();
    expect(screen.queryByText(/No failovers yet/)).not.toBeInTheDocument();

    // Cap: only the newest 20 are kept.
    act(() => {
      for (let i = 2; i <= 25; i++) {
        failoverHandler!({
          timestamp: "2026-10-28T10:00:00Z",
          from_provider: "anthropic",
          to_provider: "ollama",
          reason: `fail ${i}`,
          attempt_number: i,
        });
      }
    });
    expect(screen.getByText("↻ #25")).toBeInTheDocument();
    expect(screen.queryByText("↻ #1")).not.toBeInTheDocument();
    expect(screen.queryByText("↻ #5")).not.toBeInTheDocument();
  });

  it("unsubscribes the failover listener on unmount", async () => {
    const { unmount } = render(<ProviderSettings />);
    await waitFor(() => expect(listen).toHaveBeenCalledTimes(1));
    unmount();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it("survives a runtime without the event bridge", async () => {
    listen.mockRejectedValue(new Error("no event bridge in this runtime"));
    render(<ProviderSettings />);
    // Configs still load; the effect failure is deliberately swallowed.
    expect(await screen.findByText("Anthropic Native")).toBeInTheDocument();
    expect(screen.getByText(/No failovers yet/)).toBeInTheDocument();
  });
});
