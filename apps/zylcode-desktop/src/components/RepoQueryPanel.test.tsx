// RepoQueryPanel regression tests.
//
// Guards: the missing-selector rejection is the *caller-fixable* error and
// names what is needed; an engine rejection renders the engine's message as
// a rejection (never as empty results); an unreachable engine renders the
// guidance; a successful query renders engine-shaped result lines verbatim.
// P15 W4: the freshness line renders the stream's own states — fresh,
// stale (with the count and the re-index instruction), reindexing — and an
// unavailable signal says so instead of ever painting `fresh`; the watch is
// closed when the panel unmounts.

import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { RepoQueryPanel } from "./RepoQueryPanel";
import { runRepoQuery, type RepoQueryState } from "../lib/repoQuery";
import {
  watchRepoContext,
  type RepoContextSignal,
} from "../lib/repoContext";

vi.mock("../lib/repoQuery", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/repoQuery")>();
  return { ...actual, runRepoQuery: vi.fn() };
});
vi.mock("../lib/repoContext", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/repoContext")>();
  return { ...actual, watchRepoContext: vi.fn() };
});

const runRepoQueryMock = vi.mocked(runRepoQuery);
const watchRepoContextMock = vi.mocked(watchRepoContext);

let emitFreshness: (signal: RepoContextSignal) => void = () => {};

beforeEach(() => {
  runRepoQueryMock.mockReset();
  watchRepoContextMock.mockReset();
  watchRepoContextMock.mockImplementation((onSignal) => {
    emitFreshness = onSignal;
    return () => {};
  });
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

  // --- P15 W4: the freshness line, bound to the stream's own states -------

  it("renders a fresh standing with its short fingerprint", async () => {
    render(<RepoQueryPanel />);
    await waitFor(() => expect(watchRepoContextMock).toHaveBeenCalled());
    act(() => {
      emitFreshness({
        kind: "fresh",
        fingerprint: "a1b2c3d4e5f6",
        indexedFingerprint: "a1b2c3d4e5f6",
        changedFiles: 0,
      });
    });
    const line = screen.getByTestId("repo-query-freshness");
    expect(line.textContent).toContain("index fresh · a1b2c3d4e5f6");
  });

  it("renders stale with the change count and the re-index instruction", async () => {
    render(<RepoQueryPanel />);
    await waitFor(() => expect(watchRepoContextMock).toHaveBeenCalled());
    act(() => {
      emitFreshness({
        kind: "stale",
        fingerprint: "ffff00001111",
        indexedFingerprint: "a1b2c3d4e5f6",
        changedFiles: 2,
      });
    });
    const line = screen.getByTestId("repo-query-freshness");
    expect(line.textContent).toContain("index stale");
    expect(line.textContent).toContain("2 file(s) changed");
    expect(line.textContent).toContain("run a query to re-index");
  });

  it("renders a stale standing with no baseline as no trusted index", async () => {
    render(<RepoQueryPanel />);
    await waitFor(() => expect(watchRepoContextMock).toHaveBeenCalled());
    act(() => {
      emitFreshness({
        kind: "stale",
        fingerprint: "ffff00001111",
        indexedFingerprint: null,
        changedFiles: null,
      });
    });
    const line = screen.getByTestId("repo-query-freshness");
    expect(line.textContent).toContain("no trusted index yet");
    expect(line.textContent).not.toContain("0 file(s)");
  });

  it("renders the reindexing state while a scan is running", async () => {
    render(<RepoQueryPanel />);
    await waitFor(() => expect(watchRepoContextMock).toHaveBeenCalled());
    act(() => {
      emitFreshness({
        kind: "reindexing",
        fingerprint: "a1b2c3d4e5f6",
        indexedFingerprint: "a1b2c3d4e5f6",
        changedFiles: 0,
      });
    });
    expect(screen.getByTestId("repo-query-freshness").textContent).toContain(
      "index reindexing",
    );
  });

  it("states an unavailable signal instead of painting it fresh", async () => {
    render(<RepoQueryPanel />);
    await waitFor(() => expect(watchRepoContextMock).toHaveBeenCalled());
    act(() => {
      emitFreshness({
        kind: "unavailable",
        reason: "start `zylcode serve-intel` to stream repository freshness",
      });
    });
    const line = screen.getByTestId("repo-query-freshness");
    expect(line.textContent).toContain("unavailable");
    expect(line.textContent).toContain("serve-intel");
    // The fresh rendering is "index fresh · <hash>" — it must never appear.
    expect(line.textContent).not.toContain("index fresh ·");
  });

  it("closes the freshness watch when the panel unmounts", () => {
    const close = vi.fn();
    watchRepoContextMock.mockImplementation((onSignal) => {
      emitFreshness = onSignal;
      return close;
    });
    const { unmount } = render(<RepoQueryPanel />);
    expect(watchRepoContextMock).toHaveBeenCalled();
    unmount();
    expect(close).toHaveBeenCalledTimes(1);
  });
});
