//! Who is acting.
//!
//! # Why this exists
//!
//! `ToolContext::actor` and `ToolEvidence::actor` were added so an audit could
//! attribute an action to a person or an agent run rather than to "the app".
//! But the `Tool` trait's `call(&self, params)` has no actor parameter, so every
//! production construction site passed `None` — the field was present and
//! permanently empty.
//!
//! Widening the trait would touch every implementor, including the desktop
//! app's registrations. A task-local is the right instrument instead: "who is
//! acting" is ambient for the duration of a unit of work, exactly like a tracing
//! span.
//!
//! # Usage
//!
//! ```ignore
//! let result = zylcode_mcp::actor::with_actor("user:ada", async {
//!     engine.run_task(goal).await
//! }).await;
//! ```
//!
//! Nested scopes are allowed and the innermost wins, so an agent can set its own
//! session id inside a user-initiated request.
//!
//! An unbound task yields `None`, which the permission gate and the evidence
//! record treat as the least privileged case: unidentified.

tokio::task_local! {
    static CURRENT_ACTOR: String;
}

/// Run `future` with `actor` bound as the current actor.
///
/// The binding is task-local, so it does not leak to sibling tasks spawned
/// elsewhere. A task spawned *inside* the scope does **not** inherit it —
/// tokio task-local values do not propagate across `tokio::spawn`. The caller
/// must explicitly re-bind the actor inside the spawned closure if it wants
/// attribution to continue.
pub async fn with_actor<F, T>(actor: impl Into<String>, future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    CURRENT_ACTOR.scope(actor.into(), future).await
}

/// The actor bound to the current task, if any.
pub fn current_actor() -> Option<String> {
    CURRENT_ACTOR.try_with(|a| a.clone()).ok()
}

/// True when an actor is bound. Useful for a gate that wants to require
/// attribution for high-risk operations.
pub fn has_actor() -> bool {
    CURRENT_ACTOR.try_with(|_| ()).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unbound_tasks_have_no_actor() {
        assert_eq!(current_actor(), None);
        assert!(!has_actor());
    }

    #[tokio::test]
    async fn the_actor_is_visible_inside_the_scope() {
        let seen = with_actor("user:ada", async {
            assert!(has_actor());
            current_actor()
        })
        .await;
        assert_eq!(seen.as_deref(), Some("user:ada"));
    }

    #[tokio::test]
    async fn the_actor_does_not_leak_past_the_scope() {
        with_actor("user:ada", async {}).await;
        assert_eq!(
            current_actor(),
            None,
            "the binding must not outlive the scope"
        );
    }

    #[tokio::test]
    async fn the_innermost_scope_wins() {
        let seen = with_actor("user:ada", async {
            with_actor("agent:run-7", async { current_actor() }).await
        })
        .await;
        assert_eq!(seen.as_deref(), Some("agent:run-7"));
    }

    #[tokio::test]
    async fn sibling_tasks_are_unaffected() {
        let outer = with_actor("user:ada", async { current_actor() });
        let inner = async { current_actor() };
        let (a, b) = tokio::join!(outer, inner);
        assert_eq!(a.as_deref(), Some("user:ada"));
        assert_eq!(b, None, "a sibling future must not see the binding");
    }
}
