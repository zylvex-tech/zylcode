import React from "react";

// ---------------------------------------------------------------------------
// Theme state (components/ui/theme.tsx)
//
// Split out of `components/ui/index.tsx`. That module is the component
// barrel, and `react-refresh/only-export-components` (see
// docs/governance/ACCEPTED_LINT_WARNINGS.md) requires a component module to
// export components and nothing else — a hook or a constant alongside them
// silently disables Fast Refresh for the whole barrel.
//
// So this file owns everything that is *not* a component: the theme
// vocabulary, the catalogue, the context and the hook. `ThemeProvider` and
// `ThemeSelector` live next door in `./index.tsx` and consume it from here.
// ---------------------------------------------------------------------------

export type Theme =
  | "midnight-pro"
  | "arctic-light"
  | "github-dark"
  | "vscode-classic"
  | "solarized-dark"
  | "dracula"
  | "nord"
  | "monokai-pro";

export interface ThemeConfig {
  id: Theme;
  name: string;
  description: string;
  preview: {
    background: string;
    surface: string;
    primary: string;
    text: string;
  };
  isDark: boolean;
}

/** Available themes. */
export const themes: ThemeConfig[] = [
  {
    id: "midnight-pro",
    name: "Midnight Pro",
    description: "Deep dark theme with blue accents",
    preview: {
      background: "#0f172a",
      surface: "#1e293b",
      primary: "#3b82f6",
      text: "#f8fafc",
    },
    isDark: true,
  },
  {
    id: "arctic-light",
    name: "Arctic Light",
    description: "Clean light theme with blue accents",
    preview: {
      background: "#ffffff",
      surface: "#f8fafc",
      primary: "#2563eb",
      text: "#0f172a",
    },
    isDark: false,
  },
  {
    id: "github-dark",
    name: "GitHub Dark",
    description: "GitHub-inspired dark theme",
    preview: {
      background: "#0d1117",
      surface: "#161b22",
      primary: "#58a6ff",
      text: "#c9d1d9",
    },
    isDark: true,
  },
  {
    id: "vscode-classic",
    name: "VS Code Classic",
    description: "Visual Studio Code inspired theme",
    preview: {
      background: "#1e1e1e",
      surface: "#252526",
      primary: "#0078d4",
      text: "#cccccc",
    },
    isDark: true,
  },
  {
    id: "solarized-dark",
    name: "Solarized Dark",
    description: "Solarized dark color scheme",
    preview: {
      background: "#002b36",
      surface: "#073642",
      primary: "#268bd2",
      text: "#839496",
    },
    isDark: true,
  },
  {
    id: "dracula",
    name: "Dracula",
    description: "Popular Dracula color scheme",
    preview: {
      background: "#282a36",
      surface: "#343746",
      primary: "#bd93f9",
      text: "#f8f8f2",
    },
    isDark: true,
  },
  {
    id: "nord",
    name: "Nord",
    description: "Arctic, north-bluish color palette",
    preview: {
      background: "#2e3440",
      surface: "#3b4252",
      primary: "#88c0d0",
      text: "#eceff4",
    },
    isDark: true,
  },
  {
    id: "monokai-pro",
    name: "Monokai Pro",
    description: "Modern Monokai color scheme",
    preview: {
      background: "#2d2a2e",
      surface: "#363338",
      primary: "#fc9867",
      text: "#fcfcfa",
    },
    isDark: true,
  },
];

export interface ThemeContextType {
  currentTheme: Theme;
  setTheme: (theme: Theme) => void;
  themes: ThemeConfig[];
  isDark: boolean;
}

export const ThemeContext = React.createContext<ThemeContextType | undefined>(undefined);

/** Read the active theme. Throws outside a `ThemeProvider`. */
export function useTheme(): ThemeContextType {
  const context = React.useContext(ThemeContext);
  if (context === undefined) {
    throw new Error("useTheme must be used within a ThemeProvider");
  }
  return context;
}
