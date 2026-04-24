//! Store - Central state management with notification system
//!
//! The Store combines:
//! - Current application state
//! - Reducer for state transitions
//! - History for undo/redo
//! - Subscriber notification system

use super::actions::Action;
use super::history::History;
use super::reducers::{reduce, State};
use std::sync::Arc;

/// Subscriber callback type
pub type Subscriber = Arc<dyn Fn(&State, &Action) + Send + Sync>;

/// Subscriber ID for unsubscription
pub type SubscriberId = usize;

/// The central store that manages application state
pub struct Store {
    /// Current state
    state: State,
    /// History for undo/redo
    history: History,
    /// List of subscribers
    subscribers: Vec<(SubscriberId, Subscriber)>,
    /// Next subscriber ID
    next_subscriber_id: SubscriberId,
    /// Flag to control whether actions are recorded to history
    record_history: bool,
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

impl Store {
    /// Create a new store with default state
    pub fn new() -> Self {
        Self {
            state: State::new(),
            history: History::default(),
            subscribers: Vec::new(),
            next_subscriber_id: 0,
            record_history: true,
        }
    }

    /// Create a new store with initial state
    pub fn with_state(state: State) -> Self {
        Self {
            state,
            history: History::default(),
            subscribers: Vec::new(),
            next_subscriber_id: 0,
            record_history: true,
        }
    }

    /// Get a reference to current state
    pub fn state(&self) -> &State {
        &self.state
    }

    /// Get a clone of current state
    pub fn get_state(&self) -> State {
        self.state.clone()
    }

    /// Dispatch an action to update state
    pub fn dispatch(&mut self, action: Action) {
        // Check for undo/redo
        match &action {
            Action::Undo => {
                if let Some(previous) = self.history.undo(self.state.clone()) {
                    self.state = previous;
                    self.notify_subscribers(&action);
                }
                return;
            }
            Action::Redo => {
                if let Some(next) = self.history.redo(self.state.clone()) {
                    self.state = next;
                    self.notify_subscribers(&action);
                }
                return;
            }
            _ => {}
        }

        // Record current state to history before change
        if self.record_history && should_record_to_history(&action) {
            self.history.push(self.state.clone());
        }

        // Apply reducer
        self.state = reduce(self.state.clone(), &action);

        // Notify subscribers
        self.notify_subscribers(&action);
    }

    /// Subscribe to state changes
    /// Returns a subscriber ID that can be used to unsubscribe
    pub fn subscribe(&mut self, callback: Subscriber) -> SubscriberId {
        let id = self.next_subscriber_id;
        self.next_subscriber_id += 1;
        self.subscribers.push((id, callback));
        id
    }

    /// Unsubscribe from state changes
    pub fn unsubscribe(&mut self, id: SubscriberId) {
        self.subscribers.retain(|(sub_id, _)| *sub_id != id);
    }

    /// Clear all subscribers
    pub fn clear_subscribers(&mut self) {
        self.subscribers.clear();
    }

    /// Notify all subscribers of a state change
    fn notify_subscribers(&self, action: &Action) {
        for (_, callback) in &self.subscribers {
            callback(&self.state, action);
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// Get undo count
    pub fn undo_count(&self) -> usize {
        self.history.undo_count()
    }

    /// Get redo count
    pub fn redo_count(&self) -> usize {
        self.history.redo_count()
    }

    /// Clear history
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Execute a batch of actions without recording each to history
    /// Only the state before the batch is recorded
    pub fn dispatch_batch(&mut self, actions: Vec<Action>) {
        if actions.is_empty() {
            return;
        }

        // Record state before batch
        if self.record_history {
            self.history.push(self.state.clone());
        }

        // Apply all actions
        for action in actions {
            self.state = reduce(self.state.clone(), &action);
        }

        // Notify subscribers once after batch
        // Use a synthetic action to indicate batch completion
        self.notify_subscribers(&Action::batch(vec![]));
    }

    /// Temporarily disable history recording
    pub fn without_history<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        let prev = self.record_history;
        self.record_history = false;
        let result = f(self);
        self.record_history = prev;
        result
    }
}

/// Determine if an action should be recorded to history
fn should_record_to_history(action: &Action) -> bool {
    match action {
        // These actions should not be recorded
        Action::Undo | Action::Redo => false,
        Action::Batch(actions) => actions.iter().any(should_record_to_history),
        Action::Ui(ui_action) => {
            matches!(
                ui_action,
                super::actions::UiAction::ToggleTheme
                    | super::actions::UiAction::SetTheme(_)
                    | super::actions::UiAction::SetMode(_)
            )
        }
        Action::Message(msg_action) => {
            matches!(
                msg_action,
                super::actions::MessageAction::AddMessage(_)
                    | super::actions::MessageAction::DeleteMessage(_)
                    | super::actions::MessageAction::DeleteMessageById(_)
                    | super::actions::MessageAction::EditMessage { .. }
                    | super::actions::MessageAction::EditMessageById { .. }
            )
        }
        Action::Input(input_action) => {
            matches!(
                input_action,
                super::actions::InputAction::SubmitInput
            )
        }
        Action::Navigation(_) => false,
        Action::Search(_) => false,
        Action::Session(session_action) => {
            matches!(
                session_action,
                super::actions::SessionAction::NewSession
                    | super::actions::SessionAction::NewSessionWithName(_)
                    | super::actions::SessionAction::DeleteSession(_)
                    | super::actions::SessionAction::RenameSession(_)
            )
        }
        Action::Command(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_dispatch() {
        let mut store = Store::new();

        store.dispatch(Action::user_message("Hello"));
        assert_eq!(store.state().messages.len(), 1);

        store.dispatch(Action::user_message("World"));
        assert_eq!(store.state().messages.len(), 2);
    }

    #[test]
    fn test_store_subscribe() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Mutex;

        let mut store = Store::new();
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();

        let _id = store.subscribe(Arc::new(move |_, _| {
            call_count_clone.fetch_add(1, Ordering::SeqCst);
        }));

        store.dispatch(Action::user_message("Test"));
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        store.dispatch(Action::user_message("Test2"));
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_store_unsubscribe() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let mut store = Store::new();
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();

        let id = store.subscribe(Arc::new(move |_, _| {
            call_count_clone.fetch_add(1, Ordering::SeqCst);
        }));

        store.dispatch(Action::user_message("Test"));
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        store.unsubscribe(id);
        store.dispatch(Action::user_message("Test2"));
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_store_undo_redo() {
        let mut store = Store::new();

        store.dispatch(Action::user_message("First"));
        store.dispatch(Action::user_message("Second"));

        assert_eq!(store.state().messages.len(), 2);

        // Undo
        store.dispatch(Action::Undo);
        assert_eq!(store.state().messages.len(), 1);
        assert_eq!(store.state().messages[0].content, "First");

        // Redo
        store.dispatch(Action::Redo);
        assert_eq!(store.state().messages.len(), 2);
        assert_eq!(store.state().messages[1].content, "Second");
    }

    #[test]
    fn test_store_batch() {
        let mut store = Store::new();

        store.dispatch_batch(vec![
            Action::insert_char('a'),
            Action::insert_char('b'),
            Action::insert_char('c'),
        ]);

        assert_eq!(store.state().input, "abc");

        // Undo should undo the entire batch
        store.dispatch(Action::Undo);
        assert_eq!(store.state().input, "");
    }
}
