import { render, screen, cleanup } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { VerificationRungBadge } from "./VerificationRungBadge";
import DefaultBadge from "./VerificationRungBadge";

// ---------------------------------------------------------------------------
// Verification rung badge (components/VerificationRungBadge.tsx).
//
// The badge maps a rung number to a label, a description and a colour. Every
// rung is a distinct claim about how a build was verified, so each one is
// asserted here — including the out-of-range default, which must say
// "Unknown" rather than pretend the rung exists.
// ---------------------------------------------------------------------------

afterEach(() => cleanup());

describe("VerificationRungBadge", () => {
  const CASES: Array<[number, string, string]> = [
    [0, "Verification: None", "No verification"],
    [1, "Verification: Lint/Type-check", "Basic static analysis"],
    [2, "Verification: Property Tests", "Property-based testing"],
    [3, "Verification: Formal Spec", "Formal specification"],
    [4, "Verification: Full Formal", "Full formal verification"],
  ];

  it.each(CASES)("renders rung %i as %s", (rung, label, description) => {
    render(<VerificationRungBadge rung={rung} />);

    expect(screen.getByText(label)).toBeInTheDocument();
    expect(screen.getByText(`(${description})`)).toBeInTheDocument();
  });

  it("exposes the description as the badge tooltip", () => {
    render(<VerificationRungBadge rung={2} />);

    expect(screen.getByTitle("Property-based testing")).toBeInTheDocument();
  });

  it("reports an out-of-range rung as Unknown instead of guessing", () => {
    render(<VerificationRungBadge rung={9} />);

    expect(screen.getByText("Verification: Unknown")).toBeInTheDocument();
    expect(screen.getByText("(Unknown verification level)")).toBeInTheDocument();
  });

  it("appends the caller's extra classes to its own", () => {
    render(<VerificationRungBadge rung={1} className="mt-2" />);

    expect(screen.getByTitle("Basic static analysis").className).toContain("mt-2");
  });

  it("is reachable as a default export too, for the lazy import sites", () => {
    expect(DefaultBadge).toBe(VerificationRungBadge);
  });
});
