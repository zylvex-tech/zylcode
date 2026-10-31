import React from "react";

// ---------------------------------------------------------------------------
// Toast state (components/ui/toast.tsx)
//
// Split out of `components/ui/index.tsx` for the same reason as
// `ui/theme.tsx`: the barrel must export components and nothing else, or
// Fast Refresh stops working for every component it re-exports
// (docs/governance/ACCEPTED_LINT_WARNINGS.md).
//
// This file owns the toast vocabulary, the context and the hook;
// `ToastProvider` — the component that renders them — stays in `./index.tsx`.
// ---------------------------------------------------------------------------

export interface Toast {
  id: string;
  type: "success" | "error" | "warning" | "info";
  message: string;
}

export interface ToastContextType {
  toasts: Toast[];
  addToast: (type: Toast["type"], message: string) => void;
  removeToast: (id: string) => void;
}

export const ToastContext = React.createContext<ToastContextType | undefined>(undefined);

/** Read the toast queue. Throws outside a `ToastProvider`. */
export function useToast(): ToastContextType {
  const context = React.useContext(ToastContext);
  if (context === undefined) {
    throw new Error("useToast must be used within a ToastProvider");
  }
  return context;
}
