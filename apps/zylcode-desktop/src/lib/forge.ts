// ---------------------------------------------------------------------------
// ZylCode Forge — capability pack domain (Product Architecture v3 §14–§19)
// Domain model + strict manifest parser + taxonomy + trust labels.
// No payments, no fake packs: seeded taxonomy metadata only.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Commercial models (v3 §15) — domain enums only; no checkout exists.
// ---------------------------------------------------------------------------

export type CommercialModel =
  | "FREE"
  | "FREEMIUM"
  | "TRIAL"
  | "ONE_TIME"
  | "SUBSCRIPTION"
  | "USAGE_BASED"
  | "PER_SEAT"
  | "TEAM"
  | "ENTERPRISE";

export const COMMERCIAL_MODELS: CommercialModel[] = [
  "FREE",
  "FREEMIUM",
  "TRIAL",
  "ONE_TIME",
  "SUBSCRIPTION",
  "USAGE_BASED",
  "PER_SEAT",
  "TEAM",
  "ENTERPRISE",
];

// ---------------------------------------------------------------------------
// Marketplace taxonomy (v3 §17) — metadata only, no fake plugins.
// ---------------------------------------------------------------------------

export const FORGE_CATEGORIES = [
  { slug: "web-development", label: "Web Development" },
  { slug: "mobile", label: "Mobile" },
  { slug: "backend", label: "Backend" },
  { slug: "database", label: "Database" },
  { slug: "devops", label: "DevOps" },
  { slug: "cloud", label: "Cloud" },
  { slug: "security", label: "Security" },
  { slug: "testing", label: "Testing" },
  { slug: "ui-ux", label: "UI/UX" },
  { slug: "design-to-code", label: "Design-to-Code" },
  { slug: "ai-ml", label: "AI/ML" },
  { slug: "local-ai", label: "Local AI" },
  { slug: "mcp", label: "MCP" },
  { slug: "automation", label: "Automation" },
  { slug: "documentation", label: "Documentation" },
  { slug: "release-engineering", label: "Release Engineering" },
  { slug: "game-development", label: "Game Development" },
  { slug: "3d", label: "3D" },
  { slug: "engineering", label: "Engineering" },
  { slug: "data", label: "Data" },
  { slug: "team-enterprise", label: "Team/Enterprise" },
] as const;

export type ForgeCategorySlug = (typeof FORGE_CATEGORIES)[number]["slug"];

// Future pack concepts — taxonomy entries, NOT installed packs:
export const FORGE_FUTURE_PACKS = [
  "Android Production Engineer",
  "iOS Production Engineer",
  "React/Next Engineer",
  "Flutter Engineer",
  "Rust Systems Engineer",
  "Python Backend Engineer",
  "Database Architect",
  "DevOps Engineer",
  "Security Auditor",
  "Browser QA Engineer",
  "UI/UX Engineer",
  "Design-to-Code",
  "Visual Regression Engineer",
  "MCP Builder",
  "Plugin Builder",
  "Legacy Modernizer",
  "Codebase Archaeologist",
  "Release Readiness Auditor",
  "Unity Engineer",
  "Unreal Engineer",
  "Blender Automation",
  "AI Agent Builder",
  "Local AI Engineer",
  // Zylvex-specific engineering:
  "CAD Automation",
  "Fabrication Job Pack",
  "BOM/Cut List",
  "Drawing QA",
  "FreeCAD Automation",
  "Engineering Documentation",
  "Simulation/FEA",
  "Spatial Engineering",
] as const;

// ---------------------------------------------------------------------------
// Manifest schema (v3 §15)
// ---------------------------------------------------------------------------

export type PermissionLevel = "none" | "read" | "write" | "read-write";

export type PackPermissions = {
  filesystem: PermissionLevel;
  shell: PermissionLevel | "execute";
  network: string[]; // hosts; empty = no network
  git: PermissionLevel | "read-commit";
  browser: PermissionLevel | "control";
  secrets: string[]; // secret kinds; empty = none
  external_services: string[];
};

export type PackCommercial = {
  model: CommercialModel;
  trial: boolean | null;
  price: number | null;
  currency: string | null;
  billing_period: "monthly" | "yearly" | null;
  seats: number | null;
};

export type PackEvidence = {
  test_status: "none" | "partial" | "passed";
  verification_rung: "unverified" | "R0" | "R1" | "R2" | "R3" | "R4" | "R5";
  verified_version: string | null;
  verified_commit: string | null;
  platforms_verified: string[];
};

export type CapabilityPack = {
  id: string;
  name: string;
  publisher: string;
  version: string; // semver
  description: string;
  category: ForgeCategorySlug;
  supported_platforms: string[];
  skills: string[];
  commands: string[];
  agents: string[];
  mcp_servers: string[];
  hooks: string[];
  templates: string[];
  runtime_adapters: string[];
  permissions: PackPermissions;
  commercial: PackCommercial;
  evidence: PackEvidence;
};

// ---------------------------------------------------------------------------
// Trust labels (v3 §18) — earned by evidence, never awarded without it.
// ---------------------------------------------------------------------------

export type TrustLabel =
  | "UNVERIFIED"
  | "TESTED"
  | "VERIFIED"
  | "REPRODUCIBLE"
  | "CERTIFIED";

/**
 * Compute the trust label from manifest evidence fields. Rules:
 * - no passing tests → UNVERIFIED (regardless of claimed rung)
 * - tests passed + rung R2 → TESTED
 * - tests passed + rung R3 with verified commit → VERIFIED
 * - tests passed + rung R4 → REPRODUCIBLE
 * - rung R5 → CERTIFIED
 * A claimed rung above what evidence supports is clamped down (honesty first).
 */
export function trustLabelOf(pack: CapabilityPack): TrustLabel {
  const { test_status, verification_rung } = pack.evidence;
  if (test_status !== "passed") return "UNVERIFIED";
  switch (verification_rung) {
    case "R2":
      return "TESTED";
    case "R3":
      return pack.evidence.verified_commit ? "VERIFIED" : "TESTED";
    case "R4":
      return "REPRODUCIBLE";
    case "R5":
      return "CERTIFIED";
    default:
      return "UNVERIFIED";
  }
}

// ---------------------------------------------------------------------------
// Strict manifest parser
// ---------------------------------------------------------------------------

const SEMVER = /^\d+\.\d+\.\d+(?:-[\w.-]+)?(?:\+[\w.-]+)?$/;
const REVERSE_DNS = /^[a-z0-9-]+(?:\.[a-z0-9-]+)+$/;
const SECRET_PATTERNS = [
  /sk-[A-Za-z0-9]{16,}/, // provider key shapes
  /ghp_[A-Za-z0-9]{20,}/,
  /[A-Fa-f0-9]{32,}/,
];

export type ParseResult =
  | { ok: true; pack: CapabilityPack }
  | { ok: false; errors: string[] };

/**
 * Parse and validate a Capability Pack manifest. Strict: unknown category,
 * missing permissions/commercial/evidence blocks, bad semver, non-reverse-DNS
 * ids, or secret-shaped strings anywhere in the manifest are rejected.
 */
export function parseCapabilityPack(input: unknown): ParseResult {
  const errors: string[] = [];
  const raw = (typeof input === "object" && input !== null ? input : {}) as Record<
    string,
    unknown
  >;

  const str = (k: string): string => (typeof raw[k] === "string" ? (raw[k] as string) : "");
  const arr = (k: string): string[] =>
    Array.isArray(raw[k]) ? (raw[k] as unknown[]).filter((x): x is string => typeof x === "string") : [];

  // Secret scan over the whole serialized manifest (never embed secrets).
  const serialized = JSON.stringify(raw);
  for (const pattern of SECRET_PATTERNS) {
    if (pattern.test(serialized)) {
      errors.push("manifest appears to contain a secret-shaped string");
      break;
    }
  }

  const id = str("id");
  if (!REVERSE_DNS.test(id)) errors.push("id must be reverse-DNS (e.g. com.zylvex.pack.example)");

  const name = str("name");
  if (!name) errors.push("name is required");

  const publisher = str("publisher");
  if (!publisher) errors.push("publisher is required");

  const version = str("version");
  if (!SEMVER.test(version)) errors.push("version must be valid semver");

  const category = str("category") as ForgeCategorySlug;
  if (!FORGE_CATEGORIES.some((c) => c.slug === category)) {
    errors.push(`unknown category '${category}'`);
  }

  const supported_platforms = arr("supported_platforms");
  if (supported_platforms.length === 0) errors.push("supported_platforms must be non-empty");

  // permissions block: required, field-validated
  const permsRaw = raw["permissions"];
  let permissions: PackPermissions;
  if (typeof permsRaw !== "object" || permsRaw === null) {
    errors.push("permissions block is required");
    permissions = {
      filesystem: "none",
      shell: "none",
      network: [],
      git: "none",
      browser: "none",
      secrets: [],
      external_services: [],
    };
  } else {
    const p = permsRaw as Record<string, unknown>;
    const level = (k: string, extra: string[] = []): string => {
      const v = p[k];
      const allowed = ["none", "read", "write", "read-write", ...extra];
      if (typeof v !== "string" || !allowed.includes(v)) {
        errors.push(`permissions.${k} must be one of ${allowed.join("|")}`);
        return "none";
      }
      return v;
    };
    permissions = {
      filesystem: level("filesystem") as PermissionLevel,
      shell: level("shell", ["execute"]) as PackPermissions["shell"],
      network: Array.isArray(p["network"])
        ? (p["network"] as unknown[]).filter((x): x is string => typeof x === "string")
        : [],
      git: level("git", ["read-commit"]) as PackPermissions["git"],
      browser: level("browser", ["control"]) as PackPermissions["browser"],
      secrets: Array.isArray(p["secrets"])
        ? (p["secrets"] as unknown[]).filter((x): x is string => typeof x === "string")
        : [],
      external_services: Array.isArray(p["external_services"])
        ? (p["external_services"] as unknown[]).filter((x): x is string => typeof x === "string")
        : [],
    };
  }

  // commercial block: required; model must be known enum value
  const comRaw = raw["commercial"];
  const COMMERCIAL_LOWER = COMMERCIAL_MODELS.map((m) => m.toLowerCase());
  let commercial: PackCommercial;
  if (typeof comRaw !== "object" || comRaw === null) {
    errors.push("commercial block is required");
    commercial = {
      model: "FREE",
      trial: null,
      price: null,
      currency: null,
      billing_period: null,
      seats: null,
    };
  } else {
    const c = comRaw as Record<string, unknown>;
    const model = typeof c["model"] === "string" ? c["model"].toLowerCase() : "";
    if (!COMMERCIAL_LOWER.includes(model)) {
      errors.push(`commercial.model must be one of ${COMMERCIAL_MODELS.join("|")}`);
    }
    const bp = c["billing_period"];
    if (bp !== null && bp !== undefined && bp !== "monthly" && bp !== "yearly") {
      errors.push("commercial.billing_period must be 'monthly', 'yearly', or null");
    }
    commercial = {
      model: (model ? model.toUpperCase() : "FREE") as CommercialModel,
      trial: typeof c["trial"] === "boolean" ? c["trial"] : null,
      price: typeof c["price"] === "number" ? c["price"] : null,
      currency: typeof c["currency"] === "string" ? c["currency"] : null,
      billing_period: bp === "monthly" || bp === "yearly" ? bp : null,
      seats: typeof c["seats"] === "number" ? c["seats"] : null,
    };
  }

  // evidence block: required
  const evRaw = raw["evidence"];
  let evidence: PackEvidence;
  if (typeof evRaw !== "object" || evRaw === null) {
    errors.push("evidence block is required");
    evidence = {
      test_status: "none",
      verification_rung: "unverified",
      verified_version: null,
      verified_commit: null,
      platforms_verified: [],
    };
  } else {
    const e = evRaw as Record<string, unknown>;
    const ts = e["test_status"];
    if (ts !== "none" && ts !== "partial" && ts !== "passed") {
      errors.push("evidence.test_status must be none|partial|passed");
    }
    const rung = e["verification_rung"];
    const RUNGS = ["unverified", "R0", "R1", "R2", "R3", "R4", "R5"];
    if (typeof rung !== "string" || !RUNGS.includes(rung)) {
      errors.push(`evidence.verification_rung must be one of ${RUNGS.join("|")}`);
    }
    evidence = {
      test_status: (ts === "partial" || ts === "passed" ? ts : "none") as PackEvidence["test_status"],
      verification_rung: (typeof rung === "string" && RUNGS.includes(rung)
        ? rung
        : "unverified") as PackEvidence["verification_rung"],
      verified_version: typeof e["verified_version"] === "string" ? e["verified_version"] : null,
      verified_commit: typeof e["verified_commit"] === "string" ? e["verified_commit"] : null,
      platforms_verified: Array.isArray(e["platforms_verified"])
        ? (e["platforms_verified"] as unknown[]).filter((x): x is string => typeof x === "string")
        : [],
    };
  }

  if (errors.length > 0) return { ok: false, errors };

  return {
    ok: true,
    pack: {
      id,
      name,
      publisher,
      version,
      description: str("description"),
      category,
      supported_platforms,
      skills: arr("skills"),
      commands: arr("commands"),
      agents: arr("agents"),
      mcp_servers: arr("mcp_servers"),
      hooks: arr("hooks"),
      templates: arr("templates"),
      runtime_adapters: arr("runtime_adapters"),
      permissions,
      commercial,
      evidence,
    },
  };
}
