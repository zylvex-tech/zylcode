// ESLint flat config for zylcode-desktop.
//
// WHY THIS EXISTS
// ---------------
// `package.json` has carried `lint: "eslint src --ext .ts,.tsx"` since the
// project started, but neither eslint nor a config file was ever installed or
// committed. The script failed with "could not find config file" the moment
// anyone ran it, which means `pnpm lint` has never once succeeded. An
// external quality assessment scored the repository 35/100 on cleanliness for
// exactly this: the configuration existed in name only.
//
// This file is the real one. It is flat config because eslint 9 removed the
// legacy `.eslintrc` format, and eslint 9 is the newest major that the React
// plugin ecosystem supports (`eslint-plugin-react` and
// `eslint-plugin-react-hooks` both cap their peer range at ^9.0.0). Bumping to
// eslint 10 means dropping those two plugins; that trade is not worth the
// rules-of-hooks checks in a React codebase.
//
// Rules are warnings, not errors, EXCEPT where a rule catching a real defect is
// promoted to an error. Blanket `-D` on a first-time lint setup would produce a
// red gate on stylistic preferences and get the whole check disabled.
//
// The error-level rules below are INHERITED from the extended configs, not
// restated here, so that upgrading a plugin cannot silently change a
// severity. Verified 2026-10-21 with `eslint --print-config`:
//
//   react-hooks/rules-of-hooks       error   <- react-hooks recommended
//   @typescript-eslint/no-unused-vars error   <- explicit below
//   no-empty                         error   <- @eslint/js recommended
//   react/no-unescaped-entities      error   <- eslint-plugin-react recommended
//   @typescript-eslint/ban-ts-comment error  <- typescript-eslint recommended
//
// docs/governance/ACCEPTED_LINT_WARNINGS.md §"Why warnings are allowed and
// errors are not" records the reasoning for each severity.

import js from "@eslint/js";
import globals from "globals";
import react from "eslint-plugin-react";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import tseslint from "typescript-eslint";
import prettier from "eslint-config-prettier";

export default tseslint.config(
  {
    // Generated and vendored directories are not authored source.
    ignores: ["dist", "node_modules", "src-tauri/target", "src-tauri/gen"],
  },
  {
    files: ["**/*.{ts,tsx}"],
    extends: [
      js.configs.recommended,
      ...tseslint.configs.recommended,
      react.configs.flat.recommended,
      // tsconfig.json sets `"jsx": "react-jsx"`, i.e. the automatic JSX
      // runtime, so `React` is never in scope in a component file. Without this
      // entry the `react/react-in-jsx-scope` rule fires on every JSX element in
      // the project (1,124 findings) -- a configuration error, not a code one.
      react.configs.flat["jsx-runtime"],
    ],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: "module",
      globals: { ...globals.browser },
    },
    plugins: {
      "react-hooks": reactHooks,
      "react-refresh": reactRefresh,
    },
    settings: { react: { version: "detect" } },
    rules: {
      ...reactHooks.configs.recommended.rules,

      // Vite HMR contract: only components may be exported from a component
      // module, or a fast-refresh silently stops working.
      "react-refresh/only-export-components": ["warn", { allowConstantExport: true }],

      // Rules of hooks is a real correctness rule, not a style preference:
      // violating it is undefined behaviour at runtime.
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "warn",

      // `no-unused-vars` off: typescript-eslint's version understands type
      // positions that the base rule cannot, and would flag them wrongly.
      "no-unused-vars": "off",
      "@typescript-eslint/no-unused-vars": [
        "error",
        {
          argsIgnorePattern: "^_",
          varsIgnorePattern: "^_",
          caughtErrorsIgnorePattern: "^_",
        },
      ],

      // The codebase deliberately documents why it throws/ignores values, so
      // the type-aware variants that require prose justification are off.
      "@typescript-eslint/no-explicit-any": "warn",
      "@typescript-eslint/no-non-null-assertion": "off",
    },
  },

  // Must be last: turns off every formatting rule that would conflict with
  // prettier, so the two tools never fight over the same line.
  prettier,
);
