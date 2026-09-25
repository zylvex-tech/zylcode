// ---------------------------------------------------------------------------
// Service-backed capability views (lib/service.ts)
//
// Read-only views served by `zylcode serve-intel` so the browser preview can
// reach real capability data (tool catalogue, provider chain, token metrics)
// instead of showing hard "desktop required" stops. Secrets are never
// included: the routes themselves are secret-free by construction.
// ---------------------------------------------------------------------------

export type ToolRow = {
  id: string;
  transport: string;
  description: string | null;
  enabled: boolean;
  /** True when a real executor is bound to this id (fails closed otherwise). */
  executor_bound: boolean;
};

export type ToolsState =
  | { kind: "loading" }
  | { kind: "unavailable"; reason: string }
  | { kind: "ready"; source: string; tools: ToolRow[] };

export type ProviderRow = {
  kind: string;
  model: string;
  enabled: boolean;
  fallback_order: number;
  requires_api_key: boolean;
  timeout_ms: number;
};

export type ProvidersState =
  | { kind: "loading" }
  | { kind: "unavailable"; reason: string }
  | { kind: "ready"; primary_model: string; fallback_model: string; chain: ProviderRow[] };

export type MetricsState =
  | { kind: "loading" }
  | { kind: "unavailable"; reason: string }
  | {
      kind: "ready";
      input_tokens: number;
      output_tokens: number;
      verification_saved_tokens: number;
      fallback_count: number;
    };

async function getJson(url: string): Promise<Record<string, unknown>> {
  const response = await fetch(url, { headers: { Accept: "application/json" } });
  if (!response.ok) throw new Error(`service returned HTTP ${response.status}`);
  const payload = await response.json();
  if (payload && typeof payload === "object" && "error" in payload) {
    throw new Error(String((payload as Record<string, unknown>).error));
  }
  return payload as Record<string, unknown>;
}

export async function fetchTools(): Promise<ToolsState> {
  try {
    const data = await getJson("/api/tools");
    return {
      kind: "ready",
      source: String(data.source ?? "mcp.tools.yaml"),
      tools: (data.tools as ToolRow[]) ?? [],
    };
  } catch (e) {
    return {
      kind: "unavailable",
      reason: e instanceof Error ? e.message : "tool catalogue unavailable",
    };
  }
}

export async function fetchProviders(): Promise<ProvidersState> {
  try {
    const data = await getJson("/api/providers");
    return {
      kind: "ready",
      primary_model: String(data.primary_model ?? ""),
      fallback_model: String(data.fallback_model ?? ""),
      chain: (data.chain as ProviderRow[]) ?? [],
    };
  } catch (e) {
    return {
      kind: "unavailable",
      reason: e instanceof Error ? e.message : "provider chain unavailable",
    };
  }
}

export async function fetchMetrics(): Promise<MetricsState> {
  try {
    const data = await getJson("/api/metrics");
    return {
      kind: "ready",
      input_tokens: Number(data.input_tokens ?? 0),
      output_tokens: Number(data.output_tokens ?? 0),
      verification_saved_tokens: Number(data.verification_saved_tokens ?? 0),
      fallback_count: Number(data.fallback_count ?? 0),
    };
  } catch (e) {
    return {
      kind: "unavailable",
      reason: e instanceof Error ? e.message : "metrics unavailable",
    };
  }
}
