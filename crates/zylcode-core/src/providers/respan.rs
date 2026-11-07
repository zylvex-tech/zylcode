//! Respan hosted gateway constants.
//!
//! Respan is one OpenAI-compatible surface the router can route through. This module
//! holds the constants the router and the probe/config paths reference. The actual
//! OpenAI-compatible request/response shape is handled by the OpenAI-compatible
//! provider module; this is only the Respan-specific identifiers.

/// Base URL of the Respan hosted gateway — one OpenAI-compatible surface for every
/// upstream provider. Respan traces each request server-side, so a request that
/// arrives here is logged as a trace without any client-side tracing SDK.
///
/// [`RouterConfig::from_env`] selects [`super::super::router::ModelProvider::Respan`]
/// when `RESPAN_API_KEY` is set; [`crate::router::RouterConfig::base_url`] honours
/// `RESPAN_BASE_URL` (Respan Enterprise exposes a different endpoint).
pub const GATEWAY_BASE_URL: &str = "https://api.respan.ai/api";

/// Model used when the gateway is selected but neither `RESPAN_MODEL` nor
/// `ZYLCODE_PRIMARY_MODEL` names one. Any model the gateway serves works here — it
/// is a plain string sent in the request body.
pub const DEFAULT_MODEL: &str = "gpt-5.5";
