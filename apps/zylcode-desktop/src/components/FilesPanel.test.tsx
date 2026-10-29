import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FilesPanel } from "./FilesPanel";
import type { FileTreeResult } from "../lib/surfaces";

// ---------------------------------------------------------------------------
// Workspace files tree (components/FilesPanel.tsx).
//
// The honesty rules under test:
//   * the tree comes from the scanner via fetchFileTree — a transport failure
//     shows BLOCKED with the backend's own reason, never a fabricated tree;
//   * directories sort before files and both sort by name, and the first
//     top-level directory opens so the panel shows real structure immediately;
//   * filtering keeps ancestors of a match (a matching file must not vanish
//     with its directory) and says so when nothing matches;
//   * a plain file row is inert — only directories toggle.
// ---------------------------------------------------------------------------

vi.mock("../lib/surfaces", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/surfaces")>();
  return { ...actual, fetchFileTree: vi.fn() };
});

const { fetchFileTree } = await import("../lib/surfaces");

function tree(rows: string[]): FileTreeResult {
  return {
    total_scanned: rows.length,
    truncated: false,
    elapsed_ms: 4,
    rows: rows.map((path) => ({ path, language: "Rust" })),
  };
}

beforeEach(() => {
  vi.mocked(fetchFileTree).mockReset();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("FilesPanel", () => {
  it("reports BLOCKED with the backend's reason when the tree is unavailable", async () => {
    vi.mocked(fetchFileTree).mockResolvedValue({
      kind: "unavailable",
      reason: "start `zylcode serve-intel` to browse files",
    });
    render(<FilesPanel />);
    // BLOCKED is the initial state too — wait for the reason the backend
    // actually returned, which only lands once the fetch settles.
    expect(
      await screen.findByText("start `zylcode serve-intel` to browse files"),
    ).toBeInTheDocument();
    expect(screen.getByText("BLOCKED")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /src/ })).not.toBeInTheDocument();
  });

  it("shows LIMITED while the tree is still loading", async () => {
    vi.mocked(fetchFileTree).mockResolvedValue({ kind: "loading" });
    render(<FilesPanel />);
    expect(await screen.findByText("LIMITED")).toBeInTheDocument();
    expect(screen.getByText("loading tree…")).toBeInTheDocument();
  });

  it("renders directories before files, first directory expanded", async () => {
    vi.mocked(fetchFileTree).mockResolvedValue({
      kind: "ready",
      data: tree([
        "src/lib/util.ts",
        "README.md",
        "src/main.ts",
        "Cargo.toml",
      ]),
    });
    render(<FilesPanel />);

    expect(await screen.findByText("4 files indexed")).toBeInTheDocument();
    // Expansion is applied by an effect after the data render commits — wait
    // for a grandchild row before reading the ordering, otherwise the
    // assertion can observe the still-collapsed tree.
    expect(
      await screen.findByTitle("src/lib/util.ts"),
    ).toBeInTheDocument();

    const names = screen
      .getAllByTitle(/.*/)
      .map((el) => el.getAttribute("title"));
    // Directories first, then files, each alphabetical.
    expect(names.indexOf("src")).toBeLessThan(names.indexOf("README.md"));
    expect(names.indexOf("src")).toBeLessThan(names.indexOf("Cargo.toml"));
    expect(names.indexOf("src/lib")).toBeLessThan(names.indexOf("src/main.ts"));

    // First top-level dir auto-expands: its children are visible...
    expect(screen.getByTitle("src/lib/util.ts")).toBeInTheDocument();
    // ...and a file row is inert (no children, no toggle side effects).
    expect(screen.getByTitle("README.md")).toBeInTheDocument();
  });

  it("toggles a directory closed and open", async () => {
    vi.mocked(fetchFileTree).mockResolvedValue({
      kind: "ready",
      data: tree(["src/main.ts", "src/other.ts"]),
    });
    render(<FilesPanel />);
    const dir = (await screen.findByTitle("src")).closest("button")!;
    // Expanded on mount for the first top-level directory — the effect lands
    // a tick after the rows render, so wait for the open glyph itself.
    await waitFor(() => expect(dir.textContent).toContain("▾"));
    fireEvent.click(dir);
    expect(screen.queryByTitle("src/main.ts")).not.toBeInTheDocument();
    expect(dir.textContent).toContain("▸");
    fireEvent.click(dir);
    expect(screen.getByTitle("src/main.ts")).toBeInTheDocument();
  });

  it("filters while keeping ancestors of a match", async () => {
    vi.mocked(fetchFileTree).mockResolvedValue({
      kind: "ready",
      data: tree(["src/lib/util.ts", "src/main.ts", "README.md"]),
    });
    render(<FilesPanel />);
    await screen.findByTitle("README.md");

    fireEvent.change(screen.getByPlaceholderText("Filter files…"), {
      target: { value: "util" },
    });
    // The matching file survives...
    expect(screen.getByTitle("src/lib/util.ts")).toBeInTheDocument();
    // ...along with its ancestor chain...
    expect(screen.getByTitle("src/lib")).toBeInTheDocument();
    // ...while a non-matching sibling file is filtered out.
    expect(screen.queryByTitle("src/main.ts")).not.toBeInTheDocument();
    expect(screen.queryByTitle("README.md")).not.toBeInTheDocument();
  });

  it("says so when the filter matches nothing", async () => {
    vi.mocked(fetchFileTree).mockResolvedValue({
      kind: "ready",
      data: tree(["src/main.ts"]),
    });
    render(<FilesPanel />);
    await screen.findByTitle("src/main.ts");

    fireEvent.change(screen.getByPlaceholderText("Filter files…"), {
      target: { value: "zzz-nope" },
    });
    expect(await screen.findByText("No files match.")).toBeInTheDocument();
  });

  it("clears its interval when unmounted", async () => {
    vi.mocked(fetchFileTree).mockResolvedValue({
      kind: "ready",
      data: tree(["src/main.ts"]),
    });
    const clear = vi.spyOn(globalThis, "clearInterval");
    const { unmount } = render(<FilesPanel />);
    await waitFor(() => expect(fetchFileTree).toHaveBeenCalled());
    unmount();
    expect(clear).toHaveBeenCalled();
    clear.mockRestore();
  });
});