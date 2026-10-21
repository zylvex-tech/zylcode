/**
 * Test fixtures for the N-ATLAS SDK smoke suite.
 *
 * NOTHING IN THIS FILE IS A CREDENTIAL.
 *
 * The values exist because `redactSecrets` cannot be tested without a secret to
 * redact, and because the HTTP tests need an `apiKey` in order to prove that a
 * key never reaches evidence. They are deliberately deterministic.
 *
 * RULES:
 *   - Keep every value here obviously synthetic. Never paste a real key, and
 *     never derive one from a live account.
 *   - Do NOT read these from `process.env`. A redaction test whose input
 *     depends on the developer's shell is not a redaction test: when the
 *     variable is unset the assertion silently degrades to a no-op.
 *   - This module is test-only. `package.json#files` publishes just `src` and
 *     `README.md`, so nothing here can reach a consumer.
 */

/**
 * A deterministic, secret-shaped test input.
 *
 * `redactSecrets` performs exact-substring matching, so the value only needs to
 * be unique inside the assertion text. It deliberately avoids every real
 * provider key prefix (`sk_`, `sk_test_`, `rk_live_`, `ghp_`, `AKIA`, `AIza`,
 * `xoxb-`, `-----BEGIN ... PRIVATE KEY-----`) so that automated secret scanners
 * read it as a placeholder rather than as a leaked credential.
 */
export const TEST_FIXTURE_API_KEY = "test-fixture-not-a-key-0001";
