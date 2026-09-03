import { useState, useEffect } from "react";

export type Theme = "dark" | "oled" | "cyberpunk";

export interface ThemeConfig {
  id: Theme;
  name: string;
  bgClass: string;
  panelClass: string;
  accentClass: string;
  monacoTheme: string;
}

export const THEMES: Record<Theme, ThemeConfig> = {
  dark: {
    id: "dark",
    name: "ZylCode Dark",
    bgClass: "bg-slate-950 text-slate-100",
    panelClass: "bg-slate-900 border-slate-800",
    accentClass: "bg-cyan-500 text-slate-950",
    monacoTheme: "vs-dark",
  },
  oled: {
    id: "oled",
    name: "OLED Black",
    bgClass: "bg-black text-zinc-100",
    panelClass: "bg-zinc-950 border-zinc-900",
    accentClass: "bg-emerald-500 text-black",
    monacoTheme: "vs-dark",
  },
  cyberpunk: {
    id: "cyberpunk",
    name: "Cyberpunk",
    bgClass: "bg-zinc-950 text-fuchsia-100",
    panelClass: "bg-fuchsia-950/20 border-fuchsia-800/40",
    accentClass: "bg-fuchsia-500 text-black",
    monacoTheme: "vs-dark",
  },
};

export function useTheme() {
  const [theme, setTheme] = useState<Theme>(() => {
    return (localStorage.getItem("zylcode_theme") as Theme) || "dark";
  });

  useEffect(() => {
    localStorage.setItem("zylcode_theme", theme);
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  return { theme, setTheme, activeConfig: THEMES[theme] };
}