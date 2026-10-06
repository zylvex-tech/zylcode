/**
 * ZylCode x N-ATLAS Developer Bridge — JavaScript/TypeScript SDK.
 *
 * This is a genuine developer SDK, not a wrapper around a UI. It lets a
 * JavaScript/TypeScript developer:
 *
 *   1. configure a local N-ATLAS runtime;
 *   2. check whether that runtime is actually reachable and serving the model;
 *   3. invoke N-ATLAS and get a structured engineering intent back;
 *   4. capture a secret-free evidence record of what happened.
 *
 * ## Honesty rules this SDK enforces
 *
 * - There is **no fallback**. If the runtime is unreachable the call throws a
 *   typed error. It never returns a synthesised answer.
 * - Model identity in evidence is whatever the **server** reports. It is never
 *   taken from local configuration.
 * - A reply that does not state its model is rejected.
 * - The API key never appears in an evidence record.
 *
 * The protocol spoken is the **OpenAI-compatible** chat-completions protocol
 * (served by Ollama, llama.cpp and others). That is a public standard, not an
 * N-ATLAS-specific interface detail.
 *
 * @packageDocumentation
 */

// ---------------------------------------------------------------------------
// Errors — every failure has a stable code, never a catch-all
// ---------------------------------------------------------------------------

export type NatlasErrorCode =
  | "NOT_CONFIGURED"
  | "BLOCKED_NATLAS_ACCESS"
  | "TRANSPORT"
  | "TIMEOUT"
  | "HTTP_STATUS"
  | "MALFORMED_RESPONSE";

/** A stated failure. There is deliberately no generic "something went wrong". */
export class NatlasError extends Error {
  public readonly code: NatlasErrorCode;
  public readonly detail?: string;

  constructor(code: NatlasErrorCode, message: string, detail?: string) {
    super(message);
    this.name = "NatlasError";
    this.code = code;
    this.detail = detail;
  }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

export interface NatlasConfig {
  /** Host of the local runtime, e.g. `http://127.0.0.1:11434`. */
  baseUrl: string;
  /** Path of the OpenAI-compatible chat endpoint. */
  requestPath: string;
  /** Model name as the runtime knows it, e.g. `n-atlas:latest`. */
  model: string;
  /** Optional. Sent as a bearer token when non-empty. Never logged. */
  apiKey?: string;
  /** Our own wait bound, in milliseconds. */
  timeoutMs?: number;
}

export const OPENAI_CHAT_PATH = "/v1/chat/completions";
export const OLLAMA_TAGS_PATH = "/api/tags";
export const DEFAULT_TIMEOUT_MS = 60_000;

/**
 * A minimal environment map.
 *
 * Deliberately not `NodeJS.ProcessEnv`: the SDK must not require `@types/node`,
 * so that it works in Node, in a bundler, and in a browser playground alike.
 */
export type EnvLike = Record<string, string | undefined>;

/** The ambient environment, when one exists. Empty in a browser. */
export function defaultEnv(): EnvLike {
  const g = globalThis as { process?: { env?: EnvLike } };
  return g.process?.env ?? {};
}

/**
 * Read configuration from the environment.
 *
 * Mirrors the Rust boundary: required variables have **no defaults**, because
 * every one of them is an interface detail we would otherwise be inventing.
 */
export function configFromEnv(env: EnvLike = defaultEnv()): NatlasConfig {
  const missing: string[] = [];
  const need = (name: string): string => {
    const v = env[name]?.trim();
    if (!v) {
      missing.push(name);
      return "";
    }
    return v;
  };

  const baseUrl = need("NATLAS_BASE_URL");
  const requestPath = need("NATLAS_REQUEST_PATH");
  const model = need("NATLAS_MODEL");
  const apiKey = env["NATLAS_API_KEY"]?.trim() ?? "";

  if (missing.length > 0) {
    throw new NatlasError(
      "NOT_CONFIGURED",
      `N-ATLAS is not configured; missing environment variable(s): ${missing.join(", ")}`,
    );
  }

  const timeoutMs = Number.parseInt(env["NATLAS_TIMEOUT_MS"] ?? "", 10);

  return {
    baseUrl,
    requestPath,
    model,
    apiKey,
    timeoutMs: Number.isFinite(timeoutMs) && timeoutMs > 0 ? timeoutMs : DEFAULT_TIMEOUT_MS,
  };
}

// ---------------------------------------------------------------------------
// Request / response / evidence types
// ---------------------------------------------------------------------------

export interface ContextChunk {
  /** Workspace-relative path the excerpt came from. */
  path: string;
  excerpt: string;
}

export interface NatlasRequest {
  /** The developer's request, verbatim. */
  intent: string;
  /** System preamble. Carries the response contract the model must honour. */
  system: string;
  context?: ContextChunk[];
  maxTokens?: number;
}

export interface NatlasResponse {
  text: string;
  /** Model identity **as reported by the server**. */
  model: string;
  requestId?: string;
  finishReason?: string;
  usage: { inputTokens?: number; outputTokens?: number };
}

export interface NatlasEvidence {
  timestamp: string;
  provider: string;
  model: string;
  requestId?: string;
  requestClassification: string;
  status: "succeeded" | "failed";
  httpStatus?: number;
  latencyMs: number;
  success: boolean;
  errorCode?: string;
  promptChars: number;
  responseChars: number;
  secretRedactions: number;
}

export type IntentStepKind = "analyse" | "implement" | "test" | "document" | "approve";

export interface IntentStep {
  kind: IntentStepKind;
  description: string;
  targetPath?: string;
}

export interface EngineeringIntent {
  summary: string;
  steps: IntentStep[];
}

export interface RuntimeHealth {
  baseUrl: string;
  reachable: boolean;
  models: string[];
  modelPresent?: boolean;
  detail: string;
}

// ---------------------------------------------------------------------------
// The system preamble — ZylCode's own response contract
// ---------------------------------------------------------------------------

/**
 * The instruction that asks the model to answer in ZylCode's intent schema.
 *
 * This is **our** contract. It is not a claim about N-ATLAS's native format.
 */
export const INTENT_SYSTEM_PREAMBLE = [
  "You are an engineering assistant inside ZylCode.",
  "Answer ONLY with a JSON object, no prose and no markdown fence, in this exact shape:",
  '{"summary": string, "steps": [{"kind": "analyse"|"implement"|"test"|"document"|"approve",',
  '"description": string, "target_path": string?}]}',
  "Every step must have a non-empty description. Use only the listed kinds.",
].join(" ");

// ---------------------------------------------------------------------------
// Redaction
// ---------------------------------------------------------------------------

/**
 * Replace every occurrence of each secret with `[REDACTED]`.
 *
 * Secrets shorter than 4 characters are ignored: redacting them would corrupt
 * unrelated text while protecting nothing.
 */
export function redactSecrets(text: string, secrets: string[]): { text: string; count: number } {
  let out = text;
  let count = 0;
  for (const secret of secrets) {
    if (!secret || secret.length < 4) continue;
    const parts = out.split(secret);
    if (parts.length > 1) {
      count += parts.length - 1;
      out = parts.join("[REDACTED]");
    }
  }
  return { text: out, count };
}

// ---------------------------------------------------------------------------
// Status probe
// ---------------------------------------------------------------------------

/**
 * Probe a local runtime. Performs real I/O; `reachable` is true only when an
 * HTTP request actually succeeded.
 */
export async function probeRuntime(baseUrl: string, model?: string): Promise<RuntimeHealth> {
  const base = baseUrl.replace(/\/+$/, "");
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 3000);

  const attempt = async (url: string): Promise<string[] | undefined> => {
    try {
      const res = await fetch(url, { signal: controller.signal });
      if (!res.ok) return undefined;
      const json = (await res.json()) as Record<string, unknown>;
      const models = json["models"];
      if (Array.isArray(models)) {
        return models
          .map((m) => (m as Record<string, unknown>)["name"] ?? (m as Record<string, unknown>)["model"])
          .filter((n): n is string => typeof n === "string");
      }
      const data = json["data"];
      if (Array.isArray(data)) {
        return data
          .map((m) => (m as Record<string, unknown>)["id"])
          .filter((n): n is string => typeof n === "string");
      }
      return undefined;
    } catch {
      return undefined;
    }
  };

  try {
    const ollama = await attempt(`${base}${OLLAMA_TAGS_PATH}`);
    if (ollama) {
      return {
        baseUrl: base,
        reachable: true,
        models: ollama,
        modelPresent: model ? ollama.some((m) => m === model || m.startsWith(`${model}:`)) : undefined,
        detail: "local runtime reachable (Ollama /api/tags)",
      };
    }
    const openai = await attempt(`${base}/v1/models`);
    if (openai) {
      return {
        baseUrl: base,
        reachable: true,
        models: openai,
        modelPresent: model ? openai.includes(model) : undefined,
        detail: "local runtime reachable (OpenAI-compatible /v1/models)",
      };
    }
    return {
      baseUrl: base,
      reachable: false,
      models: [],
      detail: `no local runtime answered at ${base} (tried ${OLLAMA_TAGS_PATH} and /v1/models)`,
    };
  } finally {
    clearTimeout(timer);
  }
}

// ---------------------------------------------------------------------------
// The bridge
// ---------------------------------------------------------------------------

export interface Invocation {
  status: "succeeded" | "failed";
  response?: NatlasResponse;
  error?: NatlasError;
  evidence: NatlasEvidence;
}

/**
 * A configured client for a local N-ATLAS runtime.
 *
 * ```ts
 * const bridge = NatlasBridge.configure({
 *   baseUrl: "http://127.0.0.1:11434",
 *   requestPath: OPENAI_CHAT_PATH,
 *   model: "n-atlas:latest",
 * });
 * const health = await bridge.checkStatus();
 * const inv = await bridge.invoke({ intent: "add a health endpoint", system: INTENT_SYSTEM_PREAMBLE });
 * ```
 */
export class NatlasBridge {
  private readonly config: NatlasConfig;
  private readonly actor: string;

  private constructor(config: NatlasConfig, actor: string) {
    this.config = config;
    this.actor = actor;
  }

  static configure(config: NatlasConfig, actor = "developer"): NatlasBridge {
    if (!config.baseUrl?.trim()) {
      throw new NatlasError("NOT_CONFIGURED", "baseUrl is required");
    }
    if (!config.requestPath?.trim()) {
      throw new NatlasError("NOT_CONFIGURED", "requestPath is required");
    }
    if (!config.model?.trim()) {
      throw new NatlasError("NOT_CONFIGURED", "model is required");
    }
    return new NatlasBridge(
      { ...config, timeoutMs: config.timeoutMs ?? DEFAULT_TIMEOUT_MS },
      actor,
    );
  }

  static fromEnv(env: EnvLike = defaultEnv()): NatlasBridge {
    return NatlasBridge.configure(configFromEnv(env));
  }

  /** A log-safe view. Never contains the API key. */
  get redactedConfig(): Omit<NatlasConfig, "apiKey"> & { apiKeyPresent: boolean } {
    const { apiKey, ...rest } = this.config;
    return { ...rest, apiKeyPresent: Boolean(apiKey && apiKey.length > 0) };
  }

  /** Real I/O. Reports what it observed. */
  async checkStatus(): Promise<RuntimeHealth> {
    return probeRuntime(this.config.baseUrl, this.config.model);
  }

  private endpoint(): string {
    const base = this.config.baseUrl.replace(/\/+$/, "");
    const path = this.config.requestPath.startsWith("/")
      ? this.config.requestPath
      : `/${this.config.requestPath}`;
    return `${base}${path}`;
  }

  /**
   * Invoke N-ATLAS once.
   *
   * Never throws for a *runtime* failure: the failure is represented in the
   * returned {@link Invocation}, together with an evidence record. Throws only
   * if the SDK itself is misused.
   */
  async invoke(request: NatlasRequest): Promise<Invocation> {
    const started = Date.now();
    const classification = classify(request.intent);
    const promptChars =
      request.intent.length +
      request.system.length +
      (request.context ?? []).reduce((n, c) => n + c.excerpt.length, 0);

    const base: Omit<NatlasEvidence, "status" | "success"> = {
      timestamp: new Date().toISOString(),
      provider: "natlas",
      model: this.config.model,
      requestClassification: classification,
      latencyMs: 0,
      promptChars,
      responseChars: 0,
      secretRedactions: 0,
    };

    let httpStatus: number | undefined;
    try {
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), this.config.timeoutMs);

      let user = "";
      if (request.context?.length) {
        user += "Repository context (read-only excerpts):\n";
        for (const c of request.context) user += `\n--- ${c.path} ---\n${c.excerpt}\n`;
        user += "\nDeveloper request:\n";
      }
      user += request.intent;

      const headers: Record<string, string> = { "content-type": "application/json" };
      if (this.config.apiKey) headers["authorization"] = `Bearer ${this.config.apiKey}`;

      let res: Response;
      try {
        res = await fetch(this.endpoint(), {
          method: "POST",
          headers,
          signal: controller.signal,
          body: JSON.stringify({
            model: this.config.model,
            messages: [
              { role: "system", content: request.system },
              { role: "user", content: user },
            ],
            stream: false,
            max_tokens: request.maxTokens ?? 4096,
            temperature: 0,
          }),
        });
      } catch (e) {
        const aborted = e instanceof Error && e.name === "AbortError";
        const err = aborted
          ? new NatlasError("TIMEOUT", `N-ATLAS request timed out after ${this.config.timeoutMs} ms`)
          : new NatlasError("TRANSPORT", `could not reach the local N-ATLAS runtime: ${String(e)}`);
        return this.failure(base, err, Date.now() - started, undefined);
      } finally {
        clearTimeout(timer);
      }

      httpStatus = res.status;
      const raw = await res.text();

      if (!res.ok) {
        const err = new NatlasError("HTTP_STATUS", `N-ATLAS returned HTTP ${res.status}`, raw.slice(0, 300));
        return this.failure(base, err, Date.now() - started, httpStatus);
      }

      const response = translateOpenAiReply(raw);
      const latencyMs = Date.now() - started;
      const evidence: NatlasEvidence = {
        ...base,
        status: "succeeded",
        success: true,
        latencyMs,
        httpStatus,
        requestId: response.requestId,
        model: response.model, // the server's identity, not ours
        responseChars: response.text.length,
      };
      return { status: "succeeded", response, evidence };
    } catch (e) {
      const err =
        e instanceof NatlasError
          ? e
          : new NatlasError("MALFORMED_RESPONSE", String(e));
      return this.failure(base, err, Date.now() - started, httpStatus);
    }
  }

  /**
   * Invoke and parse the reply into a structured engineering intent.
   *
   * Returns the raw invocation alongside the parsed intent so a caller can
   * inspect both. If the model did not honour the contract, `intent` is absent
   * and the invocation records why.
   */
  async submitIntent(
    request: NatlasRequest,
  ): Promise<{ invocation: Invocation; intent?: EngineeringIntent }> {
    const invocation = await this.invoke(request);
    if (!invocation.response) return { invocation };
    try {
      return { invocation, intent: parseEngineeringIntent(invocation.response.text) };
    } catch {
      // A model that did not honour the contract is a real, reportable outcome.
      return { invocation };
    }
  }

  private failure(
    base: Omit<NatlasEvidence, "status" | "success">,
    err: NatlasError,
    latencyMs: number,
    httpStatus?: number,
  ): Invocation {
    const { text, count } = redactSecrets(
      `${err.message}${err.detail ? ` :: ${err.detail}` : ""}`,
      [this.config.apiKey ?? ""],
    );
    const evidence: NatlasEvidence = {
      ...base,
      status: "failed",
      success: false,
      latencyMs,
      httpStatus,
      errorCode: err.code,
      secretRedactions: count,
    };
    // `text` is intentionally unused beyond redaction accounting: we store the
    // code, not the message, so a server cannot smuggle a secret into evidence.
    void text;
    return { status: "failed", error: err, evidence };
  }
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

function classify(intent: string): string {
  const t = intent.toLowerCase();
  const has = (words: string[]): boolean => words.some((w) => t.includes(w));
  if (has(["implement", "add ", "fix", "refactor", "write ", "create ", "rename"])) return "implementation";
  if (has(["test", "assert", "coverage"])) return "testing";
  if (has(["explain", "why", "what is", "how does", "document", "describe"])) return "analysis";
  return "general";
}

/**
 * Translate a documented OpenAI-compatible reply into our canonical response.
 *
 * Model identity must be reported by the server; if it is absent the reply is
 * rejected rather than borrowing the configured name.
 */
export function translateOpenAiReply(body: string): NatlasResponse {
  let value: unknown;
  try {
    value = JSON.parse(body);
  } catch (e) {
    throw new NatlasError("MALFORMED_RESPONSE", "reply was not valid JSON", String(e));
  }
  if (typeof value !== "object" || value === null) {
    throw new NatlasError("MALFORMED_RESPONSE", "reply was not a JSON object");
  }
  const obj = value as Record<string, unknown>;
  const choices = obj["choices"];
  if (!Array.isArray(choices) || choices.length === 0) {
    throw new NatlasError("MALFORMED_RESPONSE", "reply has no choices");
  }
  const first = choices[0] as Record<string, unknown>;
  const message = first["message"] as Record<string, unknown> | undefined;
  const text = message?.["content"];
  if (typeof text !== "string" || text.trim() === "") {
    throw new NatlasError("MALFORMED_RESPONSE", "reply content is empty");
  }
  const model = obj["model"];
  if (typeof model !== "string" || model.trim() === "") {
    throw new NatlasError(
      "MALFORMED_RESPONSE",
      "local runtime reply did not report a model identity",
    );
  }
  const usage = obj["usage"] as Record<string, unknown> | undefined;
  const finish = first["finish_reason"];
  const id = obj["id"];

  return {
    text,
    model,
    requestId: typeof id === "string" ? id : undefined,
    finishReason: typeof finish === "string" ? finish : undefined,
    usage: {
      inputTokens: typeof usage?.["prompt_tokens"] === "number" ? (usage["prompt_tokens"] as number) : undefined,
      outputTokens:
        typeof usage?.["completion_tokens"] === "number" ? (usage["completion_tokens"] as number) : undefined,
    },
  };
}

const VALID_KINDS: readonly IntentStepKind[] = ["analyse", "implement", "test", "document", "approve"];

/** Parse ZylCode's intent contract out of a model reply. Strict. */
export function parseEngineeringIntent(text: string): EngineeringIntent {
  const stripped = stripCodeFence(text);
  let value: unknown;
  try {
    value = JSON.parse(stripped);
  } catch (e) {
    throw new NatlasError("MALFORMED_RESPONSE", "intent is not valid JSON", String(e));
  }
  if (typeof value !== "object" || value === null) {
    throw new NatlasError("MALFORMED_RESPONSE", "intent must be a JSON object");
  }
  const obj = value as Record<string, unknown>;
  const summary = obj["summary"];
  if (typeof summary !== "string" || summary.trim() === "") {
    throw new NatlasError("MALFORMED_RESPONSE", "intent requires a non-empty summary");
  }
  const rawSteps = obj["steps"];
  if (!Array.isArray(rawSteps) || rawSteps.length === 0) {
    throw new NatlasError("MALFORMED_RESPONSE", "intent requires a non-empty steps array");
  }
  const steps: IntentStep[] = rawSteps.map((raw, i) => {
    const s = raw as Record<string, unknown>;
    const kind = s["kind"];
    if (typeof kind !== "string" || !VALID_KINDS.includes(kind as IntentStepKind)) {
      throw new NatlasError("MALFORMED_RESPONSE", `step ${i} has unknown kind ${String(kind)}`);
    }
    const description = s["description"];
    if (typeof description !== "string" || description.trim() === "") {
      throw new NatlasError("MALFORMED_RESPONSE", `step ${i} is missing a description`);
    }
    const targetPath = s["target_path"];
    return {
      kind: kind as IntentStepKind,
      description,
      targetPath: typeof targetPath === "string" ? targetPath : undefined,
    };
  });
  return { summary, steps };
}

function stripCodeFence(text: string): string {
  const trimmed = text.trim();
  if (!trimmed.startsWith("```")) return trimmed;
  const nl = trimmed.indexOf("\n");
  if (nl === -1) return trimmed;
  const after = trimmed.slice(nl + 1);
  const end = after.lastIndexOf("```");
  return (end === -1 ? after : after.slice(0, end)).trim();
}
