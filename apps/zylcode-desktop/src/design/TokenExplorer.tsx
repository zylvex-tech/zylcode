// ---------------------------------------------------------------------------
// Token explorer (design/TokenExplorer.tsx)
//
// Every token the application defines, read from the stylesheet the app loads,
// with the components that use it. There is no second token database here: the
// list *is* `design-index.json`, and the values shown are the CSS values.
// ---------------------------------------------------------------------------

import { useMemo, useState } from "react";
import {
  componentsUsingToken,
  searchTokens,
  TOKEN_CATEGORIES,
  tokensOfTheme,
  usagesOfToken,
} from "./catalog";
import type { DesignIndex, DesignToken, TokenCategory } from "./types";

interface TokenExplorerProps {
  index: DesignIndex;
  theme: string;
  onTheme: (theme: string) => void;
  selected: string | null;
  onSelect: (name: string | null) => void;
  onOpenComponent: (id: string) => void;
}

const MAX_USAGES = 40;

function Swatch({ token }: { token: DesignToken }) {
  if (token.category !== "color") {
    return <span className="h-3 w-3 shrink-0 rounded-sm border border-border" aria-hidden />;
  }
  return (
    <span
      className="h-3 w-3 shrink-0 rounded-sm border border-border"
      style={{ backgroundColor: `var(${token.name})` }}
      aria-hidden
    />
  );
}

export function TokenExplorer({
  index,
  theme,
  onTheme,
  selected,
  onSelect,
  onOpenComponent,
}: TokenExplorerProps) {
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState<TokenCategory | "all">("all");

  const tokens = useMemo(() => tokensOfTheme(index, theme), [index, theme]);
  const visible = useMemo(() => searchTokens(tokens, query, category), [tokens, query, category]);
  const active = selected
    ? (index.tokens.find((t) => t.name === selected && t.theme === theme) ?? null)
    : null;
  const users = active ? componentsUsingToken(index, active.name, theme) : [];
  const sites = active ? usagesOfToken(index, active.name, theme) : [];
  const usageCount = sites.length;

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex flex-wrap items-center gap-2 border-b border-border px-3 py-2">
        <div className="flex flex-wrap gap-1" role="tablist" aria-label="Token theme">
          {["base", ...index.themes.map((t) => t.id)].map((id) => (
            <button
              key={id}
              type="button"
              role="tab"
              aria-selected={theme === id}
              onClick={() => onTheme(id)}
              className={`rounded border px-2 py-0.5 text-[10px] font-mono ${
                theme === id ? "border-primary text-primary" : "border-border text-text-muted"
              }`}
            >
              {id}
            </button>
          ))}
        </div>

        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="filter tokens…"
          aria-label="Filter tokens"
          className="ml-auto w-40 rounded border border-border bg-background px-2 py-1 text-[11px] text-text-primary"
        />

        <select
          value={category}
          onChange={(e) => setCategory(e.target.value as TokenCategory | "all")}
          aria-label="Token category"
          className="rounded border border-border bg-background px-1 py-1 text-[11px] text-text-primary"
        >
          <option value="all">all categories</option>
          {TOKEN_CATEGORIES.filter((c) => tokens.some((t) => t.category === c)).map((c) => (
            <option key={c} value={c}>
              {c}
            </option>
          ))}
        </select>
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-1 lg:grid-cols-2">
        <div className="min-h-0 overflow-auto border-r border-border" data-testid="token-list">
          <table className="w-full text-left text-[11px]">
            <thead className="sticky top-0 bg-surface text-[10px] uppercase tracking-wider text-text-muted">
              <tr>
                <th className="px-3 py-1.5 font-medium">token</th>
                <th className="px-2 py-1.5 font-medium">value</th>
                <th className="px-2 py-1.5 text-right font-medium">used by</th>
              </tr>
            </thead>
            <tbody>
              {visible.map((token) => (
                <tr
                  key={token.name}
                  onClick={() => onSelect(token.name)}
                  className={`cursor-pointer border-t border-border/50 ${
                    selected === token.name ? "bg-primary/10" : "hover:bg-surface/60"
                  }`}
                >
                  <td className="px-3 py-1 font-mono text-text-primary">
                    <span className="flex items-center gap-1.5">
                      <Swatch token={token} />
                      {token.name}
                    </span>
                  </td>
                  <td
                    className="max-w-[9rem] truncate px-2 py-1 font-mono text-text-muted"
                    title={token.value}
                  >
                    {token.value}
                  </td>
                  <td className="px-2 py-1 text-right font-mono text-text-muted">
                    {usagesOfToken(index, token.name, theme).length || "—"}
                  </td>
                </tr>
              ))}
              {visible.length === 0 && (
                <tr>
                  <td colSpan={3} className="px-3 py-6 text-center text-text-muted">
                    no token matches “{query}”
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>

        <div className="min-h-0 overflow-auto p-3" data-testid="token-detail">
          {!active ? (
            <p className="text-xs text-text-muted">
              Select a token to see its value, source and usage.
            </p>
          ) : (
            <div className="space-y-3">
              <div>
                <p className="font-mono text-sm text-text-primary">{active.name}</p>
                <p className="mt-0.5 font-mono text-[11px] text-text-muted">{active.value}</p>
                <p className="mt-1 text-[10px] font-mono text-text-muted">
                  {active.category} · {active.file}:{active.line} · theme {active.theme}
                </p>
              </div>

              <div>
                <p className="text-[10px] font-mono uppercase tracking-[0.16em] text-text-muted">
                  used by {users.length} component{users.length === 1 ? "" : "s"} ({usageCount} site
                  {usageCount === 1 ? "" : "s"})
                </p>
                {usageCount === 0 ? (
                  <p className="mt-1 text-xs text-text-muted">
                    No utility in the indexed components resolves to this token.
                  </p>
                ) : (
                  <ul className="mt-1.5 space-y-1">
                    {sites.slice(0, MAX_USAGES).map((usage) => (
                      <li key={`${usage.componentId}:${usage.line}:${usage.class}`}>
                        <button
                          type="button"
                          onClick={() => onOpenComponent(usage.componentId)}
                          className="flex w-full items-baseline justify-between gap-2 rounded border border-border/60 px-2 py-1 text-left hover:border-primary/50"
                        >
                          <span className="truncate font-mono text-[11px] text-text-primary">
                            {usage.componentId.split("#")[1] ?? usage.componentId}
                          </span>
                          <span className="shrink-0 font-mono text-[10px] text-text-muted">
                            {usage.class}:{usage.line}
                          </span>
                        </button>
                      </li>
                    ))}
                  </ul>
                )}
                {usageCount > MAX_USAGES && (
                  <p className="mt-1 text-[10px] text-text-muted">
                    showing {MAX_USAGES} of {usageCount} sites
                  </p>
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
