// ---------------------------------------------------------------------------
// Design Studio — standalone HTML export
//
// The renderer owns the markup; the Rust store writes whatever it is given. That
// keeps preview and export identical by construction instead of by two
// implementations agreeing with each other.
// ---------------------------------------------------------------------------

import type { DesignProject } from "./model";

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

/**
 * A self-contained document: no scripts, no external assets, no references to
 * the app. What the user exports opens anywhere.
 */
export function renderProjectHtml(project: DesignProject): string {
  const nodes = project.nodes
    .map((node) => {
      const style = [
        "position:absolute",
        `left:${node.x}px`,
        `top:${node.y}px`,
        `width:${node.width}px`,
        `height:${node.height}px`,
        `color:${node.color}`,
        `font-size:${node.fontSize}px`,
      ];
      if (node.kind === "rect") {
        style.push(`background:${node.color}`, "border-radius:6px");
        return `    <div style="${style.join(";")}"></div>`;
      }
      style.push("font-family:system-ui,-apple-system,'Segoe UI',sans-serif", "line-height:1.3");
      return `    <div style="${style.join(";")}">${escapeHtml(node.text)}</div>`;
    })
    .join("\n");

  return [
    "<!doctype html>",
    '<html lang="en">',
    "  <head>",
    '    <meta charset="utf-8" />',
    '    <meta name="viewport" content="width=device-width, initial-scale=1" />',
    `    <title>${escapeHtml(project.name)}</title>`,
    "  </head>",
    '  <body style="margin:0;padding:24px;background:#11151c;display:flex;justify-content:center">',
    `    <div style="position:relative;width:${project.canvas.width}px;height:${project.canvas.height}px;background:${project.canvas.background};overflow:hidden;border-radius:8px">`,
    nodes,
    "    </div>",
    "  </body>",
    "</html>",
    "",
  ].join("\n");
}

/** File name used for downloads and for the store's export path. */
export function exportFileName(project: DesignProject): string {
  const safe = project.name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return `${safe || "design"}.html`;
}
