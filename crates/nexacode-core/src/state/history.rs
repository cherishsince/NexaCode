//! Undo/Redo history management
//!
//! This module provides undo/redo functionality using a state snapshot approach.
//! It maintains a history of states that can be navigated backwards and forwards.

use super::reducers::State;
use std::collections::VecDeque;

/// Maximum number of states to keep in history
const DEFAULT_MAX_HISTORY_SIZE: usize = 50;

/// History manager for undo/redo functionality
#[derive(Debug, Clone)]
pub struct History {
    /// Past states for undo
    past: VecDeque<State>,
    /// Future states for redo
    future: VecDeque<State>,
    /// Maximum number of states to keep
    max_size: usize,
}

impl Default for History {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_HISTORY_SIZE)
    }
}

impl History {
    /// Create a new history with specified max size
    pub fn new(max_size: usize) -> Self {
        Self {
            past: VecDeque::with_capacity(max_size),
            future: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// Record a state change for potential undo
    pub fn push(&mut self, state: State) {
        // Clear redo stack when new action is taken
        self.future.clear();

        // Add to past
        if self.past.len() >= self.max_size {
            self.past.pop_front();
        }
        self.past.push_back(state);
    }

    /// Undo: move current state to future and return previous state
    pub fn undo(&mut self, current: State) -> Option<State> {
        if let Some(previous) = self.past.pop_back() {
            self.future.push_front(current);
            Some(previous)
        } else {
            None
        }
    }

    /// Redo: move current state to past and return next state
    pub fn redo(&mut self, current: State) -> Option<State> {
        if let Some(next) = self.future.pop_front() {
            self.past.push_back(current);
            Some(next)
        } else {
            None
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    /// Get the number of states that can be undone
    pub fn undo_count(&self) -> usize {
        self.past.len()
    }

    /// Get the number of states that can be redone
    pub fn redo_count(&self) -> usize {
        self.future.len()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.past.clear();
        self.future.clear();
    }

    /// Check if history is empty
    pub fn is_empty(&self) -> bool {
        self.past.is_empty() && self.future.is_empty()
    }
}

/// History configuration for different action types
#[derive(Debug, Clone, Copy)]
pub struct HistoryConfig {
    /// Whether to track this type of action
    pub track: bool,
    /// How many states to skip (for grouping related actions)
    pub skip_count: usize,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            track: true,
            skip_count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::actions::Action;

    #[test]
    fn test_history_push() {
        let mut history = History::new(10);
        let state1 = State::new();
        let state2 = State::new();

        history.push(state1);
        assert!(history.can_undo());
        assert!(!history.can_redo());

        history.push(state2);
        assert_eq!(history.undo_count(), 2);
    }

    #[test]
    fn test_history_undo_redo() {
        let mut history = History::new(10);

        let state0 = State::new();
        let mut state1 = State::new();
        state1.input = "a".to_string();
        let mut state2 = State::new();
        state2.input = "ab".to_string();

        history.push(state0.clone());
        history.push(state1.clone());

        // Undo should return previous state
        let undone = history.undo(state2.clone());
        assert!(undone.is_some());
        assert_eq!(undone.unwrap().input, "");

        assert!(history.can_redo());

        // Redo should return the state we undid from
        let redone = history.redo(state0);
        assert!(redone.is_some());
        assert_eq!(redone.unwrap().input, "ab");
    }

    #[test]
    fn test_history_max_size() {
        let mut history = History::new(3);

        for i in 0..5 {
            let mut state = State::new();
            state.input = format!("state{}", i);
            history.push(state);
        }

        assert_eq!(history.undo_count(), 3);
    }

    #[test]
    fn test_history_clear_future_on_push() {
        let mut history = History::new(10);

        let state0 = State::new();
        let mut state1 = State::new();
        state1.input = "a".to_string();
        let mut state2 = State::new();
        state2.input = "ab".to_string();
        let mut state3 = State::new();
        state3.input = "c".to_string();

        history.push(state0);
        history.push(state1.clone());

        // Undo
        let undone = history.undo(state2.clone());
        assert!(undone.is_some());
        assert!(history.can_redo());

        // Push new state should clear future
        history.push(state3);
        assert!(!history.can_redo());
    }
}
