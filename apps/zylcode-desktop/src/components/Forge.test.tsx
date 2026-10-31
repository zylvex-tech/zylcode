import { describe, expect, it } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import Forge from "./Forge";
import { FORGE_CATEGORIES, FORGE_FUTURE_PACKS } from "../lib/forge";

// ---------------------------------------------------------------------------
// Forge marketplace shell (v3 §13–§19): taxonomy browsing, honest NOT
// INSTALLED states, live manifest validation. No fake packs, no checkout.
// ---------------------------------------------------------------------------

describe("Forge", () => {
  it("renders the marketplace shell with an honest LIMITED status", () => {
    render(<Forge />);
    expect(screen.getByText("ZYLCODE FORGE")).toBeInTheDocument();
    expect(screen.getByText("LIMITED")).toBeInTheDocument();
    expect(
      screen.getByText(/taxonomy concepts, explicitly marked NOT INSTALLED/i),
    ).toBeInTheDocument();
  });

  it("lists taxonomy concept packs as NOT INSTALLED", () => {
    render(<Forge />);
    expect(screen.getAllByText("NOT INSTALLED").length).toBeGreaterThan(0);
    expect(screen.getByText("Flutter Engineer")).toBeInTheDocument();
    expect(screen.getByText("FreeCAD Automation")).toBeInTheDocument();
  });

  it("filters concept packs by search query", () => {
    render(<Forge />);
    const input = screen.getByPlaceholderText(/search capability packs/i);
    (input as HTMLInputElement).value = "";
    // Type via native setter so React state updates.
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")?.set;
    setter?.call(input, "Blender");
    input.dispatchEvent(new Event("input", { bubbles: true }));
    expect(screen.getByText("Blender Automation")).toBeInTheDocument();
    expect(screen.queryByText("Flutter Engineer")).not.toBeInTheDocument();
  });

  it("shows the empty state when nothing matches", () => {
    render(<Forge />);
    const input = screen.getByPlaceholderText(/search capability packs/i);
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")?.set;
    setter?.call(input, "zzz-no-such-pack");
    input.dispatchEvent(new Event("input", { bubbles: true }));
    expect(screen.getByText(/no concepts match the filter/i)).toBeInTheDocument();
  });

  it("seeds the full marketplace taxonomy", () => {
    // §17: 21 categories incl. Zylvex-specific engineering entries.
    expect(FORGE_CATEGORIES.length).toBe(21);
    expect(FORGE_FUTURE_PACKS).toContain("CAD Automation");
    expect(FORGE_FUTURE_PACKS).toContain("MCP Builder");
  });
});

// ---------------------------------------------------------------------------
// The three interactive surfaces the browser tests left untouched: the
// category filter, pack selection, and the live manifest validator. The
// validator is the only place Forge actually *runs* code, so its three
// outcomes (accepted / schema-rejected / not JSON) are all pinned.
// ---------------------------------------------------------------------------
describe("Forge interactions", () => {
  function openValidator() {
    render(<Forge />);
    return {
      textarea: screen.getByPlaceholderText(/^\{"id"/) as HTMLTextAreaElement,
      button: screen.getByRole("button", { name: "Validate manifest" }),
    };
  }

  it("narrows the taxonomy to the chosen category", () => {
    render(<Forge />);
    const category = FORGE_CATEGORIES[0];

    fireEvent.change(screen.getByRole("combobox"), { target: { value: category.slug } });

    expect(screen.getByRole("combobox")).toHaveValue(category.slug);
    // Concept packs are taxonomy-wide, so a real category shows none.
    expect(screen.getByText(/no concepts match the filter/i)).toBeInTheDocument();
  });

  it("opens the honest concept panel when a pack is chosen", () => {
    render(<Forge />);

    fireEvent.click(screen.getByText("Blender Automation"));

    expect(screen.getByText("CONCEPT — Blender Automation")).toBeInTheDocument();
    expect(screen.getAllByText("NOT INSTALLED").length).toBeGreaterThan(1);
  });

  it("reports malformed JSON without throwing", () => {
    const { textarea, button } = openValidator();

    fireEvent.change(textarea, { target: { value: "{not json" } });
    fireEvent.click(button);

    expect(screen.getByText(/^invalid JSON:/)).toBeInTheDocument();
    expect(screen.queryByText(/^VALID/)).not.toBeInTheDocument();
  });

  it("lists the parser's own errors for a well-formed but invalid manifest", () => {
    const { textarea, button } = openValidator();

    fireEvent.change(textarea, { target: { value: JSON.stringify({ id: "not-reverse-dns" }) } });
    fireEvent.click(button);

    expect(screen.getByText(/reverse-DNS/)).toBeInTheDocument();
    expect(screen.queryByText(/^VALID/)).not.toBeInTheDocument();
  });

  it("accepts a complete manifest through the strict parser", () => {
    const { textarea, button } = openValidator();
    const manifest = {
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
      commercial: {
        model: "FREE",
        trial: null,
        price: null,
        currency: null,
        billing_period: null,
        seats: null,
      },
      evidence: {
        test_status: "passed",
        verification_rung: "R2",
        verified_version: "1.2.3",
        verified_commit: "abc123",
        platforms_verified: ["windows"],
      },
    };

    fireEvent.change(textarea, { target: { value: JSON.stringify(manifest) } });
    fireEvent.click(button);

    expect(screen.getByText(/^VALID/)).toBeInTheDocument();
    expect(screen.queryByText(/^invalid JSON:/)).not.toBeInTheDocument();
  });
});
