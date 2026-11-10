#!/usr/bin/env node
// ---------------------------------------------------------------------------
// Design Studio source indexer
//
// Reads the application's own source and emits `src/design/generated/design-index.json`.
// The Design Studio never scans the repository at runtime: the browser has no
// filesystem, and rescanning on every interaction would make an inspector that
// is slower than reading the file. One deterministic pass produces an artefact
// that the runtime and the tests both consume.
//
// What it extracts, and how honestly:
//   * components  — exported functions/arrow components, their props (name,
//                   type, required, union values, default), source line,
//                   the classes they carry, the components they render.
//   * tokens      — from the CSS theme file that the app actually imports:
//                   every custom property, its value, its block (base scale or
//                   one of the named themes) and its line.
//   * usage       — a class name in a component maps to a token only when the
//                   Tailwind config maps that utility to that variable. A class
//                   that no mapping explains is reported as raw, never guessed.
//   * breakpoints — from the Tailwind config, not from a blog post.
//
// Uncertainty is expressed, not hidden: a prop whose type cannot be resolved is
// `unknown`, a value that is not a literal colour is not a contrast candidate,
// and a component that needs props the canvas cannot supply is `mountable: false`
// with the reason.
//
// Deterministic: sorted output, no timestamps. `--check` compares the committed
// artefact with a fresh pass and fails when they differ, so the index cannot rot.
// ---------------------------------------------------------------------------

import { readFileSync, writeFileSync, readdirSync, statSync, existsSync } from "node:fs";
import { join, relative, resolve, dirname, sep } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import ts from "typescript";

const HERE = dirname(fileURLToPath(import.meta.url));
const APP_ROOT = resolve(HERE, "..");
const SRC = join(APP_ROOT, "src");
const OUT = join(SRC, "design", "generated", "design-index.json");
const THEME_CSS = join(SRC, "styles", "themes.css");
const TAILWIND = join(APP_ROOT, "tailwind.config.js");

const check = process.argv.includes("--check");

// ---------------------------------------------------------------- file walk

function walk(dir, out = []) {
  for (const entry of readdirSync(dir).sort()) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) {
      if (entry === "generated" || entry === "node_modules" || entry === "__snapshots__") continue;
      // The Design Studio is a consumer of the index, not a subject of it.
      if (entry === "design" && dir === SRC) continue;
      walk(full, out);
    } else if (/\.tsx$/.test(entry) && !/\.test\.tsx$/.test(entry)) {
      out.push(full);
    }
  }
  return out;
}

const posix = (p) => p.split(sep).join("/");

/** `src/components/ui/index.tsx` → `components/ui/index` — unique per module. */
function relativeId(file) {
  return posix(relative(SRC, file)).replace(/\.tsx$/, "");
}

function relativePath(file) {
  return posix(relative(APP_ROOT, file));
}

// --------------------------------------------------------------- CSS tokens

/** Parse the theme stylesheet into blocks of custom properties. */
export function parseThemeCss(text) {
  const lines = text.split(/\r?\n/);
  const blocks = [];
  let current = null;

  lines.forEach((line, i) => {
    const trimmed = line.trim();
    const root = /^:root\s*\{/.exec(trimmed);
    const theme = /^\[data-theme="([^"]+)"\]\s*\{/.exec(trimmed);
    if (root) {
      current = { theme: "base", isDark: false, line: i + 1, tokens: [] };
      blocks.push(current);
      return;
    }
    if (theme) {
      current = { theme: theme[1], isDark: false, line: i + 1, tokens: [] };
      blocks.push(current);
      return;
    }
    if (trimmed === "}") {
      current = null;
      return;
    }
    if (!current) return;
    const decl = /^(--[a-z0-9-]+)\s*:\s*(.+?);\s*$/i.exec(trimmed);
    if (decl) {
      current.tokens.push({ name: decl[1], value: decl[2], line: i + 1 });
    }
  });

  return blocks;
}

/**
 * Category from the token name. The CSS file groups tokens by comment, but a
 * comment is not structure; the name is.
 */
export function tokenCategory(name) {
  const bare = name.replace(/^--/, "");
  if (bare.startsWith("color-")) return "color";
  if (bare.startsWith("space-")) return "spacing";
  if (bare.startsWith("radius-")) return "radius";
  if (bare.startsWith("text-")) return "typography";
  if (bare.startsWith("font-")) return "typography";
  if (bare.startsWith("leading-")) return "typography";
  if (bare.startsWith("shadow-")) return "shadow";
  if (bare.startsWith("transition-")) return "motion";
  if (bare.startsWith("z-")) return "depth";
  return "other";
}

// ------------------------------------------------------------ tailwind maps

async function loadTailwind() {
  const url = pathToFileURL(TAILWIND).href;
  const mod = await import(url);
  const cfg = mod.default ?? mod;
  const theme = cfg.theme?.extend ?? {};

  const colors = Object.entries(theme.colors ?? {})
    .filter(([, v]) => typeof v === "string")
    .map(([name, value]) => ({ utility: name, variable: variableOf(value) }));
  const spacing = Object.entries(theme.spacing ?? {})
    .filter(([, v]) => typeof v === "string")
    .map(([name, value]) => ({ utility: name, variable: variableOf(value) }));
  const radius = Object.entries(theme.borderRadius ?? {})
    .filter(([, v]) => typeof v === "string")
    .map(([name, value]) => ({ utility: name, variable: variableOf(value) }));
  const fontSize = Object.entries(theme.fontSize ?? {})
    .filter(([, v]) => typeof v === "string")
    .map(([name, value]) => ({ utility: name, variable: variableOf(value) }));
  const shadow = Object.entries(theme.boxShadow ?? {})
    .filter(([, v]) => typeof v === "string")
    .map(([name, value]) => ({ utility: name, variable: variableOf(value) }));

  const declared = Object.entries(cfg.theme?.screens ?? {});
  // The config does not override `screens`, so the application's breakpoints are
  // Tailwind's defaults. Say which of the two it is instead of presenting a
  // default as an application decision.
  const screens =
    declared.length > 0
      ? declared.map(([name, value]) => ({
          name,
          minWidth: typeof value === "string" ? Number(value.replace("px", "")) : Number(value),
          source: "tailwind.config.js",
        }))
      : [
          { name: "sm", minWidth: 640, source: "tailwind-default" },
          { name: "md", minWidth: 768, source: "tailwind-default" },
          { name: "lg", minWidth: 1024, source: "tailwind-default" },
          { name: "xl", minWidth: 1280, source: "tailwind-default" },
          { name: "2xl", minWidth: 1536, source: "tailwind-default" },
        ];

  return { colors, spacing, radius, fontSize, shadow, screens };
}

/** `var(--color-primary)` → `--color-primary`; anything else → null. */
function variableOf(value) {
  const m = /^var\((--[a-z0-9-]+)\)$/i.exec(String(value).trim());
  return m ? m[1] : null;
}

// --------------------------------------------------------- component source

const PRIMITIVE_TAGS = new Set([
  "button",
  "a",
  "input",
  "select",
  "textarea",
  "label",
  "img",
  "form",
  "dialog",
]);

function readProps(sourceFile, typeNode, interfaces, aliases) {
  if (!typeNode) return { props: [], unknown: false };
  let name = typeNode.getText(sourceFile).replace(/^React\./, "");

  // `React.FC<Props>` / `FC<Props>` — the props are the type argument.
  const wrapper = /^(?:React\.)?(?:FC|FunctionComponent|PropsWithChildren)<(.+)>$/.exec(name);
  if (wrapper) name = wrapper[1].trim();

  // Inline object type: `{ children: ReactNode; size?: "sm" | "md" }`
  if (name.startsWith("{")) {
    const node = typeNode;
    const members = node.members ?? [];
    return { props: members.map((m) => memberToProp(sourceFile, m, aliases)), unknown: false };
  }

  const iface = interfaces.get(name);
  if (!iface) return { props: [], unknown: true };

  const members = iface.members ?? [];
  const props = members.map((m) => memberToProp(sourceFile, m, aliases));

  // `interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement>`
  const heritage = (iface.heritageClauses ?? []).flatMap((h) =>
    h.types.map((t) => t.getText(sourceFile)),
  );
  return { props, unknown: false, extends: heritage };
}

function memberToProp(sourceFile, member, aliases) {
  if (!ts.isPropertySignature(member) || !member.name) {
    return { name: "?", type: "unknown", required: false, values: null, kind: "unknown" };
  }
  const name = member.name.getText(sourceFile).replace(/^["']|["']$/g, "");
  const optional = Boolean(member.questionToken);
  const typeText = member.type ? member.type.getText(sourceFile) : "unknown";

  const literalUnion =
    /^((?:"[^"]*"|'[^']*'|[A-Za-z0-9_]+)(\s*\|\s*(?:"[^"]*"|'[^']*'|[A-Za-z0-9_]+))*)$/.exec(
      typeText.trim(),
    );
  const inlineValues =
    literalUnion && /\|/.test(typeText)
      ? typeText
          .split("|")
          .map((v) => v.trim().replace(/^["']|["']$/g, ""))
          .filter(Boolean)
      : null;

  // `status: CapabilityStatus` is an enum in everything but syntax. Resolve the
  // alias when it is declared in the source, and stay `unknown` when it is not —
  // a guessed value list is worse than none, because the inspector would offer
  // props the component cannot accept.
  const aliasValues = !inlineValues && aliases ? aliases.get(typeText.trim()) : null;
  const values = inlineValues ?? aliasValues ?? null;

  return {
    name,
    type: typeText,
    required: !optional,
    values,
    kind: propKind(typeText, values),
  };
}

/** Exported union aliases (`type CapabilityStatus = "A" | "B"`) across the source. */
function collectAliases(files) {
  const map = new Map();
  for (const file of files) {
    const text = readFileSync(file, "utf8");
    const sf = ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
    for (const stmt of sf.statements) {
      if (!ts.isTypeAliasDeclaration(stmt)) continue;
      if (!stmt.modifiers?.some((m) => m.kind === ts.SyntaxKind.ExportKeyword)) continue;
      if (!ts.isUnionTypeNode(stmt.type)) continue;
      const values = [];
      let literalOnly = true;
      for (const t of stmt.type.types) {
        if (ts.isLiteralTypeNode(t) && ts.isStringLiteral(t.literal)) values.push(t.literal.text);
        else literalOnly = false;
      }
      if (literalOnly && values.length > 0) map.set(stmt.name.getText(sf), values);
    }
  }
  return map;
}

function propKind(typeText, values) {
  if (values) return "enum";
  const t = typeText.replace(/^React\./, "");
  if (/^(string|number|boolean)$/.test(t)) return t;
  if (/^(ReactNode|ReactElement|JSX\.Element|Element)$/.test(t)) return "node";
  if (/^\(.*\)\s*=>/.test(t)) return "function";
  if (/\[\]$/.test(t) || /^Array</.test(t)) return "array";
  return "unknown";
}

/** Split a className string into utilities, dropping template debris. */
function utilitiesOf(raw) {
  return raw
    .split(/\s+/)
    .map((c) => c.trim())
    .filter(Boolean)
    .filter((c) => /^[a-z0-9:\-/[\].%]+$/i.test(c));
}

function classAttributes(sourceFile, node, acc) {
  function visit(n) {
    if (
      ts.isJsxAttribute(n) &&
      n.name &&
      n.name.getText(sourceFile) === "className" &&
      n.initializer
    ) {
      const { line } = sourceFile.getLineAndCharacterOfPosition(n.getStart(sourceFile));
      collectInitializer(n.initializer, line + 1, acc);
    }
    ts.forEachChild(n, visit);
  }
  visit(node);
}

function collectInitializer(init, line, acc) {
  if (ts.isStringLiteral(init)) {
    acc.push({ raw: init.text, line });
  } else if (ts.isJsxExpression(init) && init.expression) {
    visitExpr(init.expression, line, acc);
  }

  function visitExpr(expr, lineNo, out) {
    if (ts.isStringLiteral(expr) || ts.isNoSubstitutionTemplateLiteral(expr)) {
      out.push({ raw: expr.text, line: lineNo });
    } else if (ts.isTemplateExpression(expr)) {
      out.push({ raw: expr.head.text, line: lineNo });
      for (const span of expr.templateSpans) out.push({ raw: span.literal.text, line: lineNo });
    } else if (ts.isConditionalExpression(expr)) {
      visitExpr(expr.whenTrue, lineNo, out);
      visitExpr(expr.whenFalse, lineNo, out);
    }
    ts.forEachChild(expr, (child) => visitExpr(child, lineNo, out));
  }
}

function a11ySignals(sourceFile, node) {
  const signals = {
    ariaLabels: 0,
    roles: 0,
    labelledControls: 0,
    imagesWithoutAlt: 0,
    clickHandlers: 0,
    nonInteractiveClickHandlers: 0,
    interactiveElements: 0,
    tabIndexes: 0,
  };

  function visit(n) {
    if (ts.isJsxOpeningElement(n) || ts.isJsxSelfClosingElement(n)) {
      const tag = n.tagName.getText(sourceFile);
      const attrs = n.attributes.properties
        .filter(ts.isJsxAttribute)
        .map((a) => a.name.getText(sourceFile));
      if (attrs.some((a) => a === "aria-label" || a === "aria-labelledby")) signals.ariaLabels += 1;
      if (attrs.includes("role")) signals.roles += 1;
      if (attrs.includes("tabIndex")) signals.tabIndexes += 1;
      if (attrs.includes("htmlFor") || attrs.includes("id")) signals.labelledControls += 1;
      if (tag === "img" && !attrs.includes("alt")) signals.imagesWithoutAlt += 1;
      if (PRIMITIVE_TAGS.has(tag)) signals.interactiveElements += 1;
      const onClick = attrs.includes("onClick");
      if (onClick) {
        signals.clickHandlers += 1;
        if (!PRIMITIVE_TAGS.has(tag)) signals.nonInteractiveClickHandlers += 1;
      }
    }
    ts.forEachChild(n, visit);
  }

  visit(node);
  return signals;
}

function importedNames(sourceFile, checker) {
  const names = new Map();
  for (const stmt of sourceFile.statements) {
    if (ts.isImportDeclaration(stmt) && stmt.importClause && stmt.moduleSpecifier) {
      const spec = stmt.moduleSpecifier.text;
      const clause = stmt.importClause;
      if (clause.name) names.set(clause.name.getText(sourceFile), spec);
      const named = clause.namedBindings;
      if (named && ts.isNamedImports(named)) {
        for (const el of named.elements) names.set(el.name.getText(sourceFile), spec);
      }
    }
  }
  void checker;
  return names;
}

/** Components rendered inside a node, resolved through the file's imports. */
function renderedComponents(sourceFile, node, imports, selfName) {
  const used = new Set();
  function visit(n) {
    if (ts.isJsxOpeningElement(n) || ts.isJsxSelfClosingElement(n)) {
      const tag = n.tagName.getText(sourceFile);
      if (/^[A-Z]/.test(tag) && tag !== selfName) used.add(tag);
    }
    ts.forEachChild(n, visit);
  }
  visit(node);
  return [...used].sort().map((name) => ({ name, from: imports.get(name) ?? "unknown" }));
}

function componentNameFromVariable(stmt) {
  if (!ts.isVariableStatement(stmt)) return null;
  const isExported = stmt.modifiers?.some((m) => m.kind === ts.SyntaxKind.ExportKeyword);
  if (!isExported) return null;
  for (const decl of stmt.declarationList.declarations) {
    if (!ts.isIdentifier(decl.name)) continue;
    const init = decl.initializer;
    if (!init) continue;
    const isArrowOrFn =
      ts.isArrowFunction(init) ||
      ts.isFunctionExpression(init) ||
      (ts.isCallExpression(init) && init.getText().startsWith("React.memo"));
    if (!isArrowOrFn) continue;
    const returnsJsx =
      /=>\s*\(?\s*</.test(init.getText()) || /return\s*\(?\s*</.test(init.getText());
    if (!returnsJsx) continue;
    return { name: decl.name.getText(), node: init, typeNode: decl.type };
  }
  return null;
}

function collectInterfaces(sourceFile) {
  const map = new Map();
  for (const stmt of sourceFile.statements) {
    if (ts.isInterfaceDeclaration(stmt)) map.set(stmt.name.getText(sourceFile), stmt);
    if (ts.isTypeAliasDeclaration(stmt) && ts.isTypeLiteralNode(stmt.type)) {
      map.set(stmt.name.getText(sourceFile), stmt.type);
    }
  }
  return map;
}

function indexFile(file, tailwind, aliases) {
  const text = readFileSync(file, "utf8");
  const sourceFile = ts.createSourceFile(
    file,
    text,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX,
  );
  const interfaces = collectInterfaces(sourceFile);
  const imports = importedNames(sourceFile, null);
  const components = [];

  const pushComponent = (name, node, typeNode, line, kind) => {
    const classes = [];
    classAttributes(sourceFile, node, classes);
    const props = readProps(sourceFile, typeNode, interfaces, aliases);
    // The same class can appear twice on one line (`pl-10 pr-10` in a template),
    // and a duplicate is not two usages of anything.
    const seen = new Set();
    const utilities = [];
    for (const entry of classes) {
      for (const utility of utilitiesOf(entry.raw)) {
        const key = `${utility}@${entry.line}`;
        if (seen.has(key)) continue;
        seen.add(key);
        utilities.push({ utility, line: entry.line });
      }
    }
    components.push({
      id: `${relativeId(file)}#${name}`,
      name,
      file: relativePath(file),
      line,
      kind,
      props: props.props,
      propsUnknown: props.unknown === true,
      extends: props.extends ?? [],
      children: renderedComponents(sourceFile, node, imports, name),
      classes: utilities.map((u) => u.utility),
      classSites: utilities.map((u) => ({ utility: u.utility, line: u.line })),
      a11y: a11ySignals(sourceFile, node),
      tokens: utilities.map((u) => utilityToken(u.utility, tailwind)).filter((t) => t !== null),
    });
  };

  for (const stmt of sourceFile.statements) {
    const exported =
      ts.isFunctionDeclaration(stmt) &&
      stmt.name &&
      stmt.modifiers?.some((m) => m.kind === ts.SyntaxKind.ExportKeyword);

    if (exported) {
      const { line } = sourceFile.getLineAndCharacterOfPosition(stmt.getStart(sourceFile));
      const name = stmt.name.getText(sourceFile);
      pushComponent(name, stmt, stmt.parameters[0]?.type, line + 1, classify(file, name));
      continue;
    }

    const variable = componentNameFromVariable(stmt);
    if (variable) {
      const { line } = sourceFile.getLineAndCharacterOfPosition(variable.node.getStart(sourceFile));
      const params = variable.node.parameters ?? [];
      pushComponent(
        variable.name,
        variable.node,
        params[0]?.type ?? variable.typeNode,
        line + 1,
        classify(file, variable.name),
      );
    }
  }

  return { components };
}

function classify(file, name) {
  const rel = posix(relative(SRC, file));
  if (rel === "App.tsx") return "page";
  if (rel.startsWith("components/shell/") && /Surface|Host|Shell/.test(name)) return "screen";
  if (rel.startsWith("components/ui/")) return "design-system";
  return "component";
}

/** Map one Tailwind utility to the CSS variable the config binds it to. */
export function utilityToken(utility, tailwind) {
  const normalized = utility.replace(/^[a-z-]+:/, "");

  const colorPrefixes = [
    "bg-",
    "text-",
    "border-",
    "ring-",
    "from-",
    "via-",
    "to-",
    "fill-",
    "stroke-",
    "placeholder-",
    "divide-",
  ];
  for (const prefix of colorPrefixes) {
    if (!normalized.startsWith(prefix)) continue;
    let rest = normalized.slice(prefix.length);
    rest = rest.replace(/\/(\d+|\[[^\]]+\])$/, "");
    const hit = tailwind.colors.find((c) => c.utility === rest);
    if (hit?.variable) return hit.variable;
  }

  const spacingPrefixes = [
    "p-",
    "px-",
    "py-",
    "pt-",
    "pb-",
    "pl-",
    "pr-",
    "m-",
    "mx-",
    "my-",
    "mt-",
    "mb-",
    "ml-",
    "mr-",
    "gap-",
    "space-x-",
    "space-y-",
    "w-",
    "h-",
    "min-w-",
    "min-h-",
    "max-w-",
    "max-h-",
    "inset-",
  ];
  for (const prefix of spacingPrefixes) {
    if (!normalized.startsWith(prefix)) continue;
    const rest = normalized.slice(prefix.length);
    const hit = tailwind.spacing.find((s) => s.utility === rest);
    if (hit?.variable) return hit.variable;
  }

  if (normalized.startsWith("rounded")) {
    const rest = normalized === "rounded" ? "DEFAULT" : normalized.replace(/^rounded-/, "");
    const hit = tailwind.radius.find((r) => r.utility === rest);
    if (hit?.variable) return hit.variable;
  }
  if (normalized.startsWith("text-")) {
    const rest = normalized.replace(/^text-/, "");
    const hit = tailwind.fontSize.find((f) => f.utility === rest);
    if (hit?.variable) return hit.variable;
  }
  if (normalized.startsWith("shadow")) {
    const rest = normalized === "shadow" ? "DEFAULT" : normalized.replace(/^shadow-/, "");
    const hit = tailwind.shadow.find((s) => s.utility === rest);
    if (hit?.variable) return hit.variable;
  }
  return null;
}

/** Screens the app declares, taken from the shell's own switch statement. */
function indexScreens(files) {
  const host = files.find((f) => f.endsWith(join("shell", "SurfaceHost.tsx")));
  if (!host) return [];
  const text = readFileSync(host, "utf8");
  const names = [...text.matchAll(/case "([a-z-]+)":/g)].map((m) => m[1]);
  return [...new Set(names)].sort().map((name) => ({
    id: name,
    name,
    file: relativePath(host),
    // A screen is the shell's view, and it needs the application's state to
    // render. The index says so instead of offering a canvas that would throw.
    mountable: false,
    reason: "composed from application state; inspect its source and components instead",
  }));
}

// --------------------------------------------------------------------- build

async function build() {
  const tailwind = await loadTailwind();
  const files = walk(SRC);

  const aliases = collectAliases(files);
  const components = [];
  for (const file of files) {
    components.push(...indexFile(file, tailwind, aliases).components);
  }
  components.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));

  const themeCss = readFileSync(THEME_CSS, "utf8");
  const blocks = parseThemeCss(themeCss);

  const baseTokens = blocks
    .filter((b) => b.theme === "base")
    .flatMap((b) =>
      b.tokens.map((t) => ({
        name: t.name,
        value: t.value,
        category: tokenCategory(t.name),
        theme: "base",
        file: relativePath(THEME_CSS),
        line: t.line,
      })),
    );

  const themeTokenCount = blocks
    .filter((b) => b.theme !== "base")
    .reduce((n, b) => n + b.tokens.length, 0);

  const themes = blocks
    .filter((b) => b.theme !== "base")
    .map((b) => ({
      id: b.theme,
      isDark: /dark/i.test(b.theme),
      line: b.line,
      tokenCount: b.tokens.length,
    }));

  const themeTokens = blocks
    .filter((b) => b.theme !== "base")
    .flatMap((b) =>
      b.tokens.map((t) => ({
        name: t.name,
        value: t.value,
        category: tokenCategory(t.name),
        theme: b.theme,
        file: relativePath(THEME_CSS),
        line: t.line,
      })),
    );

  const usage = new Map();
  for (const component of components) {
    for (const site of component.classSites) {
      const token = utilityToken(site.utility, tailwind);
      if (!token) continue;
      if (!usage.has(token)) usage.set(token, []);
      usage.get(token).push({ componentId: component.id, class: site.utility, line: site.line });
    }
  }

  const tokens = [...baseTokens, ...themeTokens].sort((a, b) =>
    a.theme === b.theme
      ? a.name < b.name
        ? -1
        : a.name > b.name
          ? 1
          : 0
      : a.theme < b.theme
        ? -1
        : 1,
  );

  // A variable name is shared by every theme that defines it, so usages are
  // stored once per name instead of once per (name, theme). On this codebase
  // that is the difference between a 1.3 MB artefact and a much smaller one.
  const usages = {};
  for (const name of [...usage.keys()].sort()) {
    usages[name] = usage
      .get(name)
      .sort((a, b) =>
        a.componentId === b.componentId ? a.line - b.line : a.componentId < b.componentId ? -1 : 1,
      );
  }

  const index = {
    contract: "design-studio-index/1",
    sourceRoot: "apps/zylcode-desktop/src",
    note: "Generated by scripts/design-index.mjs from the application's own source. Do not edit by hand.",
    stats: {
      files: files.length,
      components: components.filter((c) => c.kind === "component").length,
      designSystemComponents: components.filter((c) => c.kind === "design-system").length,
      screens: components.filter((c) => c.kind === "screen").length,
      pages: components.filter((c) => c.kind === "page").length,
      themes: themes.length,
      baseTokens: baseTokens.length,
      themeTokens: themeTokenCount,
      tokens: tokens.length,
      tokenUsages: Object.values(usages).reduce((n, v) => n + v.length, 0),
    },
    breakpoints: tailwind.screens,
    themes,
    tokens,
    usages,
    components,
    screens: indexScreens(files),
    tailwind: {
      colors: tailwind.colors,
      spacing: tailwind.spacing,
      radius: tailwind.radius,
      fontSize: tailwind.fontSize,
      shadow: tailwind.shadow,
    },
  };

  return index;
}

const index = await build();
// Format with the repository's own Prettier config so the artefact passes
// `format:check` without a second, hand-tuned serialiser.
const prettier = await import("prettier");
const prettierConfig = await prettier.resolveConfig(OUT);
const serialized = await prettier.format(JSON.stringify(index), {
  ...prettierConfig,
  parser: "json",
});

if (check) {
  if (!existsSync(OUT)) {
    console.error(`design index missing: ${relativePath(OUT)} (run node scripts/design-index.mjs)`);
    process.exit(1);
  }
  const current = readFileSync(OUT, "utf8");
  if (current !== serialized) {
    console.error(
      `design index is stale: ${relativePath(OUT)} differs from a fresh index of the source.\n` +
        `run: node scripts/design-index.mjs`,
    );
    process.exit(1);
  }
  console.log(
    `design index ok: ${index.stats.components + index.stats.designSystemComponents} components, ` +
      `${index.stats.tokens} tokens, ${index.stats.themes} themes`,
  );
} else {
  writeFileSync(OUT, serialized, "utf8");
  console.log(
    `design index written: ${relativePath(OUT)}\n` +
      `  components ${index.stats.components + index.stats.designSystemComponents} ` +
      `(design-system ${index.stats.designSystemComponents}), screens ${index.stats.screens}, ` +
      `pages ${index.stats.pages}\n` +
      `  tokens ${index.stats.tokens} (base ${index.stats.baseTokens} + theme ${index.stats.themeTokens}), ` +
      `themes ${index.stats.themes}, usages ${index.stats.tokenUsages}\n` +
      `  breakpoints ${index.breakpoints.map((b) => `${b.name}@${b.minWidth}`).join(" ")}`,
  );
}
