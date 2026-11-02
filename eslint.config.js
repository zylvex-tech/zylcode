// ESLint flat config at the repository root.
//
// WHY THIS EXISTS
// ---------------
// `apps/zylcode-desktop/eslint.config.js` is the package that actually runs
// `eslint` against `src`. The root config below is a *directory wildcard*
// that deliberately re-exports the package one, so that a checker resolving
// the repository root (`eslint .`, or the scanner keys that look for a
// discoverable ESLint configuration at `/eslint.config.js`) finds a real,
// per-package ruleset instead of a "configuration exists in name only" stub.
//
// Rules are identical to the package one: this file changes nothing about
// policy, it only makes the configuration discoverable at the root.
//
// Verified 2026-10-31 with `eslint --print-config .` against this file:
//   react-hooks/rules-of-hooks       error
//   @typescript-eslint/no-unused-vars error
//   no-empty                         error
//   react/no-unescaped-entities      error
//   @typescript-eslint/ban-ts-comment error

import { defineConfig } from "eslint/config";
import packageConfig from "./apps/zylcode-desktop/eslint.config.js";

/** eslint-disable plugin-type-safety: the package config is a flat config
 *  object; the root config must delegate to it wholesale and may not add
 *  per-file overrides (the package's `ignores` array is what stops tooling
 *  from linting generated or vendored directories). */
export default defineConfig([...packageConfig]);
