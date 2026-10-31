import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { LiveSearch } from "./LiveSearch";
import type { SearchResult } from "../lib/surfaces";

// ---------------------------------------------------------------------------
// Live search (components/LiveSearch.tsx).
//
// The rules under test:
//   * nothing is prefetched — the idle copy says results come from the real
//     index, and no query fires until the user asks (or initialQuery does);
//   * an empty query cannot be submitted;
//   * results carry relevance, resource, type and the retriever's own reason,
//     so a hit is auditable rather than a bare filename;
//   * failure states (unavailable/empty) are rendered as themselves.
// ---------------------------------------------------------------------------

vi.mock("../lib/surfaces", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/surfaces")>();
  return { ...actual, fetchSearch: vi.fn() };
});

const { fetchSearch } = await import("../lib/surfaces");

function ready(overrides: Partial<SearchResult> = {}): SearchResult {
  return {
    query: "where is the scanner",
    elapsed_ms: 12.4,
    indexed_files: 612,
    results: [
      {
        resource: "crates/zylcode-core/src/intelligence/scanner.rs",
        resource_type: "file",
        relevance: 1.75,
        reason: "path token 'scanner'",
      },
    ],
    ...overrides,
  };
}

beforeEach(() => {
  vi.mocked(fetchSearch).mockReset();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("LiveSearch", () => {
  it("starts idle: BLOCKED badge, no prefetch, submit disabled", () => {
    render(<LiveSearch />);
    expect(screen.getByText("BLOCKED")).toBeInTheDocument();
    expect(
      screen.getByText("Enter a query — results come from the real index, ranked with provenance."),
    ).toBeInTheDocument();
    expect(fetchSearch).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Search" })).toBeDisabled();
  });

  it("runs a submitted query and renders ranked, explained results", async () => {
    vi.mocked(fetchSearch).mockResolvedValue({ kind: "ready", data: ready() });
    render(<LiveSearch />);

    fireEvent.change(screen.getByPlaceholderText("Search symbols, files, packages…"), {
      target: { value: "where is the scanner" },
    });
    const submit = screen.getByRole("button", { name: "Search" });
    expect(submit).toBeEnabled();
    fireEvent.click(submit);

    expect(await screen.findByText("1.75")).toBeInTheDocument();
    expect(fetchSearch).toHaveBeenCalledWith("where is the scanner");
    expect(screen.getByText("AVAILABLE")).toBeInTheDocument();
    expect(screen.getByText("612 files indexed · 12 ms")).toBeInTheDocument();
    expect(screen.getByText("crates/zylcode-core/src/intelligence/scanner.rs")).toBeInTheDocument();
    expect(screen.getByText("[file]")).toBeInTheDocument();
    expect(screen.getByText("path token 'scanner'")).toBeInTheDocument();
  });

  it("labels itself LIMITED while searching and disables the button", async () => {
    let resolveSearch: (v: Awaited<ReturnType<typeof fetchSearch>>) => void = () => {};
    vi.mocked(fetchSearch).mockReturnValue(
      new Promise((r) => {
        resolveSearch = r;
      }),
    );
    render(<LiveSearch />);
    fireEvent.change(screen.getByPlaceholderText("Search symbols, files, packages…"), {
      target: { value: "scanner" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Search" }));

    expect(await screen.findByText("LIMITED")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Searching…" })).toBeDisabled();

    resolveSearch({ kind: "ready", data: ready() });
    await waitFor(() => expect(screen.getByText("AVAILABLE")).toBeInTheDocument());
  });

  it("refuses to search an empty or whitespace query", async () => {
    render(<LiveSearch />);
    const input = screen.getByPlaceholderText("Search symbols, files, packages…");
    fireEvent.change(input, { target: { value: "   " } });
    fireEvent.submit(input.closest("form")!);
    expect(fetchSearch).not.toHaveBeenCalled();
  });

  it("reports an empty result set as empty", async () => {
    vi.mocked(fetchSearch).mockResolvedValue({
      kind: "ready",
      data: ready({ results: [], query: "nothing here" }),
    });
    render(<LiveSearch />);
    fireEvent.change(screen.getByPlaceholderText("Search symbols, files, packages…"), {
      target: { value: "nothing here" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Search" }));
    expect(await screen.findByText("No results for “nothing here”.")).toBeInTheDocument();
  });

  it("surfaces the backend's own unavailability reason", async () => {
    vi.mocked(fetchSearch).mockResolvedValue({
      kind: "unavailable",
      reason: "intelligence service is not running",
    });
    render(<LiveSearch />);
    fireEvent.change(screen.getByPlaceholderText("Search symbols, files, packages…"), {
      target: { value: "scanner" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Search" }));
    expect(await screen.findByText("intelligence service is not running")).toBeInTheDocument();
    expect(screen.getByText("BLOCKED")).toBeInTheDocument();
  });

  it("runs an initialQuery on mount without user input", async () => {
    vi.mocked(fetchSearch).mockResolvedValue({ kind: "ready", data: ready() });
    render(<LiveSearch initialQuery="scanner" />);
    await waitFor(() => expect(fetchSearch).toHaveBeenCalledWith("scanner"));
    expect(screen.getByPlaceholderText("Search symbols, files, packages…")).toHaveValue("scanner");
  });
});
