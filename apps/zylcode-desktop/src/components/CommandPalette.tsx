import React, { useEffect, useMemo, useRef, useState } from "react";
import type { Menu } from "./shell/MenuBar";

interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
  /** Flat command list derived from the menu bar. */
  menus: Menu[];
}

interface FlatCommand {
  id: string;
  menu: string;
  label: string;
  shortcut?: string;
  run?: () => void;
  disabled?: boolean;
  disabledReason?: string;
}

/**
 * Command palette over the real menu commands. Disabled commands appear
 * with their reason and cannot run — same honesty as the menus.
 */
export const CommandPalette: React.FC<CommandPaletteProps> = ({ open, onClose, menus }) => {
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const commands: FlatCommand[] = useMemo(
    () =>
      menus.flatMap((m) =>
        m.items
          .filter((i) => !i.separatorBefore)
          .map((i) => ({
            id: `${m.id}.${i.id}`,
            menu: m.label,
            label: i.label,
            shortcut: i.shortcut,
            run: i.run,
            disabled: i.disabled,
            disabledReason: i.disabledReason,
          })),
      ),
    [menus],
  );

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return commands;
    return commands.filter(
      (c) => c.label.toLowerCase().includes(q) || c.menu.toLowerCase().includes(q),
    );
  }, [commands, query]);

  useEffect(() => {
    if (open) {
      setQuery("");
      setSelected(0);
      setTimeout(() => inputRef.current?.focus(), 10);
    }
  }, [open]);

  useEffect(() => setSelected(0), [query]);

  if (!open) return null;

  const execute = (c: FlatCommand) => {
    if (c.disabled) return;
    onClose();
    c.run?.();
  };

  return (
    <div
      className="fixed inset-0 z-[100] bg-black/50 backdrop-blur-sm flex items-start justify-center pt-[12vh] p-4"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="w-full max-w-lg bg-surface border border-border rounded-lg shadow-2xl overflow-hidden">
        <input
          ref={inputRef}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") onClose();
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setSelected((s) => Math.min(s + 1, filtered.length - 1));
            }
            if (e.key === "ArrowUp") {
              e.preventDefault();
              setSelected((s) => Math.max(s - 1, 0));
            }
            if (e.key === "Enter" && filtered[selected]) {
              execute(filtered[selected]);
            }
          }}
          placeholder="Type a command…"
          className="w-full px-4 py-3 bg-transparent text-sm text-text-primary placeholder:text-text-muted focus:outline-none border-b border-border"
        />
        <div className="max-h-[50vh] overflow-y-auto py-1">
          {filtered.length === 0 && (
            <p className="px-4 py-3 text-xs text-text-muted">No matching commands.</p>
          )}
          {filtered.map((c, i) => (
            <button
              key={c.id}
              onClick={() => execute(c)}
              onMouseEnter={() => setSelected(i)}
              title={c.disabled ? c.disabledReason : undefined}
              className={`w-full text-left px-4 py-2 flex items-center gap-2 text-xs ${
                i === selected ? "bg-primary/10" : ""
              } ${c.disabled ? "opacity-50 cursor-not-allowed" : "text-text-primary"}`}
            >
              <span className="text-[10px] text-text-muted font-mono w-14 shrink-0">{c.menu}</span>
              <span className="truncate">{c.label}</span>
              {c.shortcut && (
                <span className="ml-auto text-[10px] text-text-muted font-mono shrink-0">{c.shortcut}</span>
              )}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
};

export default CommandPalette;
