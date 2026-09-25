import React from "react";

export interface MenuItem {
  id: string;
  label: string;
  shortcut?: string;
  /** When set, the command runs for real. */
  run?: () => void;
  /** When true, the item renders disabled with the reason as its tooltip. */
  disabled?: boolean;
  disabledReason?: string;
  separatorBefore?: boolean;
  checked?: boolean;
}

export interface Menu {
  id: string;
  label: string;
  items: MenuItem[];
}

interface MenuBarProps {
  menus: Menu[];
  /** Brand area rendered left of the menus (logo, wordmark, version). */
  brand?: React.ReactNode;
  /** Context area rendered right of the menus. */
  right?: React.ReactNode;
}

/**
 * The top menu bar: real commands, honest disabled states.
 * A disabled item always carries a `disabledReason` explaining what is
 * missing; nothing here pretends to work.
 */
export const MenuBar: React.FC<MenuBarProps> = ({ menus, brand, right }) => {
  const [openMenu, setOpenMenu] = React.useState<string | null>(null);
  const barRef = React.useRef<HTMLDivElement>(null);

  React.useEffect(() => {
    if (!openMenu) return;
    const close = (e: MouseEvent) => {
      if (barRef.current && !barRef.current.contains(e.target as Node)) {
        setOpenMenu(null);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpenMenu(null);
    };
    document.addEventListener("mousedown", close);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", close);
      document.removeEventListener("keydown", onKey);
    };
  }, [openMenu]);

  return (
    <div
      ref={barRef}
      className="relative flex items-center gap-1 px-2 h-9 shrink-0 border-b border-border bg-surface/90 select-none"
      role="menubar"
    >
      {brand}
      {menus.map((menu) => {
        const open = openMenu === menu.id;
        return (
          <div key={menu.id} className="relative">
            <button
              role="menuitem"
              aria-haspopup="menu"
              aria-expanded={open}
              onClick={() => setOpenMenu(open ? null : menu.id)}
              onMouseEnter={() => openMenu && setOpenMenu(menu.id)}
              className={`px-2.5 py-1 rounded text-xs transition-colors ${
                open
                  ? "bg-primary/15 text-primary"
                  : "text-text-secondary hover:text-text-primary hover:bg-surface-hover"
              }`}
            >
              {menu.label}
            </button>
            {open && (
              <div
                role="menu"
                aria-label={menu.label}
                className="absolute left-0 top-full mt-1 z-50 min-w-[260px] rounded-md border border-border bg-surface shadow-xl py-1"
              >
                {menu.items.map((item) =>
                  item.separatorBefore ? (
                    <div key={`sep-${item.id}`} className="my-1 border-t border-border" role="separator" />
                  ) : null,
                ).concat(
                  menu.items
                    .filter((i) => !i.separatorBefore)
                    .map((item) => (
                      <button
                        key={item.id}
                        role="menuitem"
                        disabled={item.disabled}
                        title={item.disabled ? item.disabledReason : undefined}
                        onClick={() => {
                          if (item.disabled) return;
                          setOpenMenu(null);
                          item.run?.();
                        }}
                        className={`w-full text-left px-3 py-1.5 text-xs flex items-center justify-between gap-6 ${
                          item.disabled
                            ? "text-text-muted/50 cursor-not-allowed"
                            : "text-text-primary hover:bg-primary/10"
                        }`}
                      >
                        <span className="flex items-center gap-1.5">
                          {item.checked !== undefined && (
                            <span className="w-3 text-primary">{item.checked ? "✓" : ""}</span>
                          )}
                          {item.label}
                        </span>
                        {item.shortcut && (
                          <span className="text-[10px] text-text-muted font-mono">{item.shortcut}</span>
                        )}
                      </button>
                    )),
                )}
              </div>
            )}
          </div>
        );
      })}
      <div className="ml-auto flex items-center gap-2">{right}</div>
    </div>
  );
};

export default MenuBar;
