// RepoQueryPanel regression tests.
//
// Guards: the missing-selector rejection is the *caller-fixable* error and
// names what is needed; an engine rejection renders the engine's message as
// a rejection (never as empty results); an unreachable engine renders the
// guidance; a successful query renders engine-shaped result lines verbatim.

import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { RepoQueryPanel } from "./RepoQueryPanel";
import { runRepoQuery, type RepoQueryState } from "../lib/repoQuery";

vi.mock("../lib/repoQuery", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/repoQuery")>();
  return { ...actual, runRepoQuery: vi.fn() };
});

const runRepoQueryMock = vi.mocked(runRepoQuery);

beforeEach(() => {
  runRepoQueryMock.mockReset();
});

const READY: RepoQueryState = {
  kind: "ready",
  via: "desktop",
  data: {
    method: "find_symbol",
    arg: "Engine",
    elapsed_ms: 12,
    results: [
      {
        resource: "crates/zylcode-core/src/lib.rs",
        resource_type: "symbol",
        relevance: 0.9,
        reason: "symbol name match",
        provenance: "DETERMINISTIC",
        snippet: null,
      },
    ],
  },
};

describe("RepoQueryPanel", () => {
  it("runs a query and renders engine results verbatim", async () => {
    runRepoQueryMock.mockResolvedValue(READY);
    render(<RepoQueryPanel />);
    await act(async () => {
      fireEvent.click(screen.getByTestId("repo-query-run"));
    });
    expect(runRepoQueryMock).toHaveBeenCalledWith("find_symbol", "");
    await waitFor(() =>
      expect(screen.getByTestId("repo-query-results").textContent).toContain(
        "crates/zylcode-core/src/lib.rs",
      ),
    );
  });

  it("renders an engine rejection as a rejection, not empty results", async () => {
    runRepoQueryMock.mockResolvedValue({
      kind: "rejected",
      reason: "method 'find_symbol' requires a symbol name (arg)",
    });
    render(<RepoQueryPanel />);
    await act(async () => {
      fireEvent.click(screen.getByTestId("repo-query-run"));
    });
    const alert = screen.getByRole("alert");
    expect(alert.getAttribute("data-testid")).toBe("repo-query-rejected");
    expect(alert.textContent).toContain("requires a symbol name");
    expect(screen.queryByTestId("repo-query-results")).toBeNull();
  });

  it("renders an unreachable engine with its guidance", async () => {
    runRepoQueryMock.mockResolvedValue({
      kind: "unavailable",
      reason: "start `zylcode serve-intel` to run repository queries",
    });
    render(<RepoQueryPanel />);
    await act(async () => {
      fireEvent.click(screen.getByTestId("repo-query-run"));
    });
    expect(screen.getByTestId("repo-query-unavailable").textContent).toContain(
      "serve-intel",
    );
  });

  it("changes the selector hint when a stateless method is picked", async () => {
    render(<RepoQueryPanel />);
    fireEvent.change(screen.getByTestId("repo-query-method"), {
      target: { value: "entry_points" },
    });
    expect(
      (screen.getByTestId("repo-query-arg") as HTMLInputElement).disabled,
    ).toBe(true);
    expect(
      (screen.getByTestId("repo-query-arg") as HTMLInputElement).placeholder,
    ).toBe("no selector needed");
  });
});
