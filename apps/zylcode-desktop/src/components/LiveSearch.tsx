import { useCallback, useEffect, useState } from "react";
import { Panel } from "./Panel";
import { StatusBadge, type CapabilityStatus } from "./CapabilityStatus";
import { fetchSearch, type SearchResult } from "../lib/surfaces";

/**
 * Live Search surface: ranked results from the real intelligence pipeline
 * (indexed symbols/files plus path matches). No query runs until asked —
 * nothing is prefetched or fabricated.
 */
export function LiveSearch({ initialQuery = "" }: { initialQuery?: string }) {
  const [query, setQuery] = useState(initialQuery);
  const [state, setState] = useState<
    { kind: "idle" } | { kind: "loading" } | { kind: "unavailable"; reason: string } | { kind: "ready"; data: SearchResult }
  >({ kind: "idle" });

  const run = useCallback((q: string) => {
    setState({ kind: "loading" });
    let cancelled = false;
    fetchSearch(q).then((next) => {
      if (!cancelled) setState(next.kind === "loading" ? { kind: "idle" } : next);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (initialQuery.trim().length > 0) {
      run(initialQuery);
    }
  }, [initialQuery, run]);

  const headerStatus: CapabilityStatus =
    state.kind === "ready" ? "AVAILABLE" : state.kind === "loading" ? "LIMITED" : "BLOCKED";

  return (
    <Panel title="SEARCH">
      <div className="space-y-3">
        <div className="flex items-center justify-between gap-2">
          <StatusBadge status={headerStatus} size="xs" />
          {state.kind === "ready" && (
            <span className="text-[10px] font-mono text-text-muted">
              {state.data.indexed_files} files indexed · {state.data.elapsed_ms.toFixed(0)} ms
            </span>
          )}
        </div>

        <form
          className="flex gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            if (query.trim().length > 0) run(query);
          }}
        >
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search symbols, files, packages…"
            className="input text-sm flex-1"
          />
          <button
            type="submit"
            className="btn btn-primary text-xs"
            disabled={state.kind === "loading" || query.trim().length === 0}
          >
            {state.kind === "loading" ? "Searching…" : "Search"}
          </button>
        </form>

        {state.kind === "unavailable" && <p className="text-xs text-text-muted">{state.reason}</p>}
        {state.kind === "idle" && (
          <p className="text-xs text-text-muted">
            Enter a query — results come from the real index, ranked with provenance.
          </p>
        )}

        {state.kind === "ready" && (
          <>
            {state.data.results.length === 0 ? (
              <p className="text-xs text-text-muted">No results for “{state.data.query}”.</p>
            ) : (
              <ol className="space-y-1.5">
                {state.data.results.map((r) => (
                  <li key={`${r.resource_type}:${r.resource}`} className="text-xs">
                    <span className="font-mono text-[10px] text-text-muted">
                      {r.relevance.toFixed(2)}
                    </span>{" "}
                    <span className="font-mono">{r.resource}</span>{" "}
                    <span className="text-[10px] text-text-muted">[{r.resource_type}]</span>
                    <p className="text-[10px] text-text-muted truncate">{r.reason}</p>
                  </li>
                ))}
              </ol>
            )}
          </>
        )}
      </div>
    </Panel>
  );
}
