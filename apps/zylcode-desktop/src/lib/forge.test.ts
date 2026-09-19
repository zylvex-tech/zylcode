import { describe, expect, it } from "vitest";
import { parseCapabilityPack, trustLabelOf, type CapabilityPack } from "./forge";

// ---------------------------------------------------------------------------
// Capability Pack manifest parser + trust label computation (v3 §14–§19).
// Trust labels are earned by evidence; over-claims are clamped down.
// ---------------------------------------------------------------------------

function validManifest(): Record<string, unknown> {
  return {
    id: "com.zylvex.pack.example",
    name: "Example Pack",
    publisher: "Zylvex",
    version: "1.2.3",
    description: "A test pack.",
    category: "automation",
    supported_platforms: ["windows", "linux"],
    skills: [],
    commands: [],
    agents: [],
    mcp_servers: [],
    hooks: [],
    templates: [],
    runtime_adapters: [],
    permissions: {
      filesystem: "read-write",
      shell: "execute",
      network: ["github.com"],
      git: "read-commit",
      browser: "none",
      secrets: [],
      external_services: [],
    },
    commercial: { model: "FREE", trial: null, price: null, currency: null, billing_period: null, seats: null },
    evidence: {
      test_status: "passed",
      verification_rung: "R2",
      verified_version: "1.2.3",
      verified_commit: "abc123",
      platforms_verified: ["windows"],
    },
  };
}

describe("parseCapabilityPack", () => {
  it("accepts a complete valid manifest", () => {
    const res = parseCapabilityPack(validManifest());
    expect(res.ok).toBe(true);
    if (res.ok) {
      expect(res.pack.id).toBe("com.zylvex.pack.example");
      expect(res.pack.permissions.filesystem).toBe("read-write");
      expect(res.pack.commercial.model).toBe("FREE");
      expect(res.pack.evidence.verification_rung).toBe("R2");
    }
  });

  it("rejects a non-reverse-DNS id", () => {
    const m = validManifest();
    m.id = "not-reverse-dns";
    const res = parseCapabilityPack(m);
    expect(res.ok).toBe(false);
    if (!res.ok) expect(res.errors.some((e) => e.includes("reverse-DNS"))).toBe(true);
  });

  it("rejects invalid semver", () => {
    const m = validManifest();
    m.version = "not-semver";
    const res = parseCapabilityPack(m);
    expect(res.ok).toBe(false);
    if (!res.ok) expect(res.errors.some((e) => e.includes("semver"))).toBe(true);
  });

  it("rejects unknown categories", () => {
    const m = validManifest();
    m.category = "not-a-category";
    const res = parseCapabilityPack(m);
    expect(res.ok).toBe(false);
    if (!res.ok) expect(res.errors.some((e) => e.includes("unknown category"))).toBe(true);
  });

  it("requires the permissions block", () => {
    const m = validManifest();
    delete m.permissions;
    const res = parseCapabilityPack(m);
    expect(res.ok).toBe(false);
    if (!res.ok) expect(res.errors.some((e) => e.includes("permissions block"))).toBe(true);
  });

  it("requires the evidence block", () => {
    const m = validManifest();
    delete m.evidence;
    const res = parseCapabilityPack(m);
    expect(res.ok).toBe(false);
    if (!res.ok) expect(res.errors.some((e) => e.includes("evidence block"))).toBe(true);
  });

  it("rejects invalid permission levels", () => {
    const m = validManifest();
    m.permissions = { ...(m.permissions as object), filesystem: "superuser" };
    const res = parseCapabilityPack(m);
    expect(res.ok).toBe(false);
    if (!res.ok) expect(res.errors.some((e) => e.includes("permissions.filesystem"))).toBe(true);
  });

  it("rejects manifests containing secret-shaped strings", () => {
    const m = validManifest();
    m.description = "key sk-abcdef0123456789abcdef0123456789 embedded";
    const res = parseCapabilityPack(m);
    expect(res.ok).toBe(false);
    if (!res.ok) expect(res.errors.some((e) => e.includes("secret"))).toBe(true);
  });

  it("normalizes commercial.model case", () => {
    const m = validManifest();
    m.commercial = { model: "subscription", trial: true, price: 9, currency: "USD", billing_period: "monthly", seats: 5 };
    const res = parseCapabilityPack(m);
    expect(res.ok).toBe(true);
    if (res.ok) expect(res.pack.commercial.model).toBe("SUBSCRIPTION");
  });

  it("rejects non-object input", () => {
    const res = parseCapabilityPack("nope");
    expect(res.ok).toBe(false);
  });
});

describe("trustLabelOf", () => {
  function packWith(evidence: Partial<CapabilityPack["evidence"]>): CapabilityPack {
    const res = parseCapabilityPack(validManifest());
    if (!res.ok) throw new Error("fixture must parse");
    return { ...res.pack, evidence: { ...res.pack.evidence, ...evidence } };
  }

  it("is UNVERIFIED without passing tests — even if a rung is claimed", () => {
    expect(trustLabelOf(packWith({ test_status: "none", verification_rung: "R4" }))).toBe("UNVERIFIED");
    expect(trustLabelOf(packWith({ test_status: "partial", verification_rung: "R3" }))).toBe("UNVERIFIED");
  });

  it("is TESTED for passed tests at R2", () => {
    expect(trustLabelOf(packWith({ test_status: "passed", verification_rung: "R2" }))).toBe("TESTED");
  });

  it("is VERIFIED for R3 only with a verified commit", () => {
    expect(trustLabelOf(packWith({ test_status: "passed", verification_rung: "R3", verified_commit: "deadbeef" }))).toBe("VERIFIED");
    expect(trustLabelOf(packWith({ test_status: "passed", verification_rung: "R3", verified_commit: null }))).toBe("TESTED");
  });

  it("is REPRODUCIBLE at R4 and CERTIFIED at R5", () => {
    expect(trustLabelOf(packWith({ verification_rung: "R4" }))).toBe("REPRODUCIBLE");
    expect(trustLabelOf(packWith({ verification_rung: "R5" }))).toBe("CERTIFIED");
  });

  it("is UNVERIFIED for R0/R1 even with passing tests", () => {
    expect(trustLabelOf(packWith({ verification_rung: "R0" }))).toBe("UNVERIFIED");
    expect(trustLabelOf(packWith({ verification_rung: "R1" }))).toBe("UNVERIFIED");
  });
});
