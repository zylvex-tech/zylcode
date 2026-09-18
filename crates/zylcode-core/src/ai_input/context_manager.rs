use chrono::{DateTime, Utc};

use super::types::*;

/// Context manager for maintaining conversation and processing context
pub struct ContextManager {
    current_context: InputContext,
    context_history: Vec<ContextSnapshot>,
    max_history_size: usize,
}

/// A point-in-time record of the input context.
///
/// The history is currently append-only: entries are written on every
/// `update_context` / `clear_context`, and only the *count* is ever read (via
/// `ContextStats::context_history_size`). The payload is therefore stored but
/// never consumed — class B (unfinished). Fields keep their values; they are
/// `_`-prefixed to record that no reader exists yet.
#[derive(Debug, Clone)]
struct ContextSnapshot {
    _context: InputContext,
    _timestamp: DateTime<Utc>,
    _trigger: String,
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextManager {
    /// Create a new context manager
    pub fn new() -> Self {
        Self {
            current_context: InputContext::default(),
            context_history: Vec::new(),
            max_history_size: 100,
        }
    }

    /// Get current context
    pub fn get_current_context(&self) -> InputContext {
        self.current_context.clone()
    }

    /// Update context
    pub fn update_context(&mut self, new_context: InputContext) {
        // Save current context to history
        self.context_history.push(ContextSnapshot {
            _context: self.current_context.clone(),
            _timestamp: Utc::now(),
            _trigger: "context_update".to_string(),
        });

        // Trim history if needed
        if self.context_history.len() > self.max_history_size {
            self.context_history.remove(0);
        }

        // Update current context
        self.current_context = new_context;
    }

    /// Add to context history
    pub fn add_to_history(&mut self, input: ProcessedInput) {
        self.current_context.history.push(input);
        
        // Keep only last 50 inputs in history
        if self.current_context.history.len() > 50 {
            self.current_context.history.remove(0);
        }
    }

    /// Get context history
    pub fn get_history(&self) -> Vec<ProcessedInput> {
        self.current_context.history.clone()
    }

    /// Get recent context (last N inputs)
    pub fn get_recent_context(&self, n: usize) -> Vec<ProcessedInput> {
        let history = &self.current_context.history;
        let start = if history.len() > n { history.len() - n } else { 0 };
        history[start..].to_vec()
    }

    /// Update user preferences
    pub fn update_preferences(&mut self, key: String, value: String) {
        self.current_context.preferences.insert(key, value);
    }

    /// Get preference
    pub fn get_preference(&self, key: &str) -> Option<&String> {
        self.current_context.preferences.get(key)
    }

    /// Set language
    pub fn set_language(&mut self, language: String) {
        self.current_context.language = language;
    }

    /// Set timezone
    pub fn set_timezone(&mut self, timezone: String) {
        self.current_context.timezone = timezone;
    }

    /// Set user ID
    pub fn set_user_id(&mut self, user_id: String) {
        self.current_context.user_id = Some(user_id);
    }

    /// Set session ID
    pub fn set_session_id(&mut self, session_id: String) {
        self.current_context.session_id = Some(session_id);
    }

    /// Set project ID
    pub fn set_project_id(&mut self, project_id: String) {
        self.current_context.project_id = Some(project_id);
    }

    /// Clear context
    pub fn clear_context(&mut self) {
        // Save current context to history before clearing
        self.context_history.push(ContextSnapshot {
            _context: self.current_context.clone(),
            _timestamp: Utc::now(),
            _trigger: "context_clear".to_string(),
        });

        // Reset to default
        self.current_context = InputContext::default();
    }

    /// Get context statistics
    pub fn get_stats(&self) -> ContextStats {
        ContextStats {
            history_size: self.current_context.history.len(),
            context_history_size: self.context_history.len(),
            preferences_count: self.current_context.preferences.len(),
            language: self.current_context.language.clone(),
            timezone: self.current_context.timezone.clone(),
        }
    }
}

/// Context statistics
#[derive(Debug, Clone)]
pub struct ContextStats {
    pub history_size: usize,
    pub context_history_size: usize,
    pub preferences_count: usize,
    pub language: String,
    pub timezone: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_manager() {
        let mut manager = ContextManager::new();
        
        // Test initial state
        let context = manager.get_current_context();
        assert_eq!(context.language, "en");
        assert_eq!(context.timezone, "UTC");
        
        // Test update
        let new_context = InputContext {
            language: "fr".to_string(),
            ..InputContext::default()
        };
        manager.update_context(new_context);
        
        let context = manager.get_current_context();
        assert_eq!(context.language, "fr");
        
        // Test preferences
        manager.update_preferences("theme".to_string(), "dark".to_string());
        assert_eq!(manager.get_preference("theme"), Some(&"dark".to_string()));
        
        // Test stats
        let stats = manager.get_stats();
        assert_eq!(stats.language, "fr");
    }
}