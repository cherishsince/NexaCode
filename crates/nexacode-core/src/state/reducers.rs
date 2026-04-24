//! State reducers
//!
//! Reducers are pure functions that take the current state and an action,
//! and return a new state. They should have no side effects.

use super::actions::{
    now_timestamp, Action, AgentState, CommandAction, FocusTarget,
    InputAction, Message, MessageAction, Mode, NavigationAction, SearchAction,
    SessionAction, Timestamp, UiAction,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Theme configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

/// Search match result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    /// Message index
    pub message_index: usize,
    /// Start position in content
    pub start: usize,
    /// End position in content
    pub end: usize,
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session ID
    pub id: String,
    /// Session name (optional)
    pub name: Option<String>,
    /// Creation timestamp
    pub created_at: Timestamp,
    /// Last modified timestamp
    pub modified_at: Timestamp,
    /// Messages in this session
    pub messages: Vec<Message>,
}

impl Session {
    pub fn new() -> Self {
        let now = now_timestamp();
        Self {
            id: generate_session_id(),
            name: None,
            created_at: now,
            modified_at: now,
            messages: Vec::new(),
        }
    }

    pub fn with_name(name: impl Into<String>) -> Self {
        let mut session = Self::new();
        session.name = Some(name.into());
        session
    }

    pub fn touch(&mut self) {
        self.modified_at = now_timestamp();
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a unique session ID
fn generate_session_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = now_timestamp();
    format!("sess_{}_{}", timestamp, count)
}

/// Generate a unique message ID (public for Session use)
pub fn generate_message_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = now_timestamp();
    format!("msg_{}_{}", timestamp, count)
}

/// The central application state
#[derive(Debug, Clone)]
pub struct State {
    // ========================================
    // Session State
    // ========================================
    /// Current active session
    pub current_session: Session,
    /// All sessions (session_id -> Session)
    pub sessions: HashMap<String, Session>,

    // ========================================
    // Message State (convenience accessors)
    // ========================================
    /// Conversation messages (reference to current_session.messages)
    /// This is kept for compatibility and quick access
    pub messages: Vec<Message>,
    /// Input history (previous inputs for navigation)
    pub input_history: Vec<String>,
    /// Current position in input history (for up/down navigation)
    pub input_history_index: Option<usize>,

    // ========================================
    // Input State
    // ========================================
    /// Current input buffer
    pub input: String,
    /// Cursor position in input buffer (byte position)
    pub cursor_pos: usize,
    /// Saved input when navigating history
    pub saved_input: String,

    // ========================================
    // Navigation State
    // ========================================
    /// Scroll offset in message list
    pub scroll_offset: usize,

    // ========================================
    // Search State
    // ========================================
    /// Current search query
    pub search_query: Option<String>,
    /// Search results (message indices with match positions)
    pub search_results: Vec<SearchMatch>,
    /// Current match index in search results
    pub current_match_index: Option<usize>,
    /// Whether search is case sensitive
    pub case_sensitive: bool,

    // ========================================
    // Command State
    // ========================================
    /// Command history
    pub command_history: Vec<String>,
    /// Current position in command history
    pub command_history_index: Option<usize>,
    /// Saved command when navigating history
    pub saved_command: String,

    // ========================================
    // UI State
    // ========================================
    /// Current agent operational state
    pub agent_state: AgentState,
    /// Current application mode
    pub mode: Mode,
    /// Current theme
    pub theme: Theme,
    /// Whether to show help overlay
    pub show_help: bool,
    /// Currently focused UI element
    pub focus: FocusTarget,
    /// Whether the application should quit
    pub should_quit: bool,
    /// Status message to display
    pub status_message: Option<String>,
    /// Whether status message is an error
    pub status_is_error: bool,
}

impl Default for State {
    fn default() -> Self {
        let session = Session::new();
        let mut sessions = HashMap::new();
        sessions.insert(session.id.clone(), session.clone());

        Self {
            current_session: session,
            sessions,
            messages: Vec::new(),
            input_history: Vec::new(),
            input_history_index: None,
            input: String::new(),
            cursor_pos: 0,
            saved_input: String::new(),
            scroll_offset: 0,
            search_query: None,
            search_results: Vec::new(),
            current_match_index: None,
            case_sensitive: false,
            command_history: Vec::new(),
            command_history_index: None,
            saved_command: String::new(),
            agent_state: AgentState::Idle,
            mode: Mode::Input,
            theme: Theme::Light,
            show_help: false,
            focus: FocusTarget::Input,
            should_quit: false,
            status_message: None,
            status_is_error: false,
        }
    }
}

impl State {
    /// Create a new state with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new state with dark theme
    pub fn dark() -> Self {
        Self {
            theme: Theme::Dark,
            ..Self::default()
        }
    }

    /// Check if there are any messages
    pub fn has_messages(&self) -> bool {
        !self.messages.is_empty()
    }

    /// Get the number of messages
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Get the last message
    pub fn last_message(&self) -> Option<&Message> {
        self.messages.last()
    }

    /// Find message by ID
    pub fn find_message_by_id(&self, id: &str) -> Option<(usize, &Message)> {
        self.messages
            .iter()
            .enumerate()
            .find(|(_, m)| m.id == id)
    }

    /// Check if input is empty
    pub fn is_input_empty(&self) -> bool {
        self.input.trim().is_empty()
    }

    /// Get input content
    pub fn input_content(&self) -> &str {
        &self.input
    }

    /// Check if agent is busy
    pub fn is_agent_busy(&self) -> bool {
        self.agent_state != AgentState::Idle
    }

    /// Check if in input mode
    pub fn is_input_mode(&self) -> bool {
        self.mode == Mode::Input
    }

    /// Check if in command mode
    pub fn is_command_mode(&self) -> bool {
        self.mode == Mode::Command
    }

    /// Check if in search mode
    pub fn is_search_mode(&self) -> bool {
        self.mode == Mode::Search
    }

    /// Check if in normal mode
    pub fn is_normal_mode(&self) -> bool {
        self.mode == Mode::Normal
    }

    /// Check if search is active
    pub fn is_searching(&self) -> bool {
        self.search_query.is_some() && !self.search_results.is_empty()
    }

    /// Get current match if any
    pub fn current_match(&self) -> Option<&SearchMatch> {
        self.current_match_index
            .and_then(|idx| self.search_results.get(idx))
    }

    /// Get total number of search matches
    pub fn match_count(&self) -> usize {
        self.search_results.len()
    }

    /// Sync messages with current session
    fn sync_messages(&mut self) {
        self.messages = self.current_session.messages.clone();
    }
}

/// The main reducer function that handles all actions
pub fn reduce(state: State, action: &Action) -> State {
    match action {
        Action::Message(msg_action) => reduce_message(state, msg_action),
        Action::Input(input_action) => reduce_input(state, input_action),
        Action::Navigation(nav_action) => reduce_navigation(state, nav_action),
        Action::Ui(ui_action) => reduce_ui(state, ui_action),
        Action::Search(search_action) => reduce_search(state, search_action),
        Action::Session(session_action) => reduce_session(state, session_action),
        Action::Command(cmd_action) => reduce_command(state, cmd_action),
        Action::Undo | Action::Redo => {
            // These are handled by the Store, not the reducer
            state
        }
        Action::Batch(actions) => actions.iter().fold(state, |s, a| reduce(s, a)),
    }
}

/// Reduce message-related actions
fn reduce_message(mut state: State, action: &MessageAction) -> State {
    match action {
        MessageAction::AddMessage(message) => {
            state.current_session.messages.push(message.clone());
            state.current_session.touch();
            state.sync_messages();
            // Auto-scroll to bottom when new message is added
            state.scroll_offset = state.messages.len().saturating_sub(1);
        }
        MessageAction::ClearMessages => {
            state.current_session.messages.clear();
            state.current_session.touch();
            state.sync_messages();
            state.scroll_offset = 0;
            // Clear search if any
            state.search_query = None;
            state.search_results.clear();
            state.current_match_index = None;
        }
        MessageAction::DeleteMessage(index) => {
            if *index < state.current_session.messages.len() {
                state.current_session.messages.remove(*index);
                state.current_session.touch();
                state.sync_messages();
                // Adjust scroll if necessary
                if state.scroll_offset >= state.messages.len() {
                    state.scroll_offset = state.messages.len().saturating_sub(1);
                }
            }
        }
        MessageAction::DeleteMessageById(id) => {
            if let Some(pos) = state.current_session.messages.iter().position(|m| m.id == *id) {
                state.current_session.messages.remove(pos);
                state.current_session.touch();
                state.sync_messages();
            }
        }
        MessageAction::EditMessage { index, content } => {
            if let Some(msg) = state.current_session.messages.get_mut(*index) {
                msg.content = content.clone();
                msg.timestamp = now_timestamp(); // Update timestamp on edit
            }
            state.current_session.touch();
            state.sync_messages();
        }
        MessageAction::EditMessageById { id, content } => {
            if let Some(msg) = state.current_session.messages.iter_mut().find(|m| m.id == *id) {
                msg.content = content.clone();
                msg.timestamp = now_timestamp();
            }
            state.current_session.touch();
            state.sync_messages();
        }
    }
    state
}

/// Reduce input-related actions
fn reduce_input(mut state: State, action: &InputAction) -> State {
    match action {
        InputAction::InsertChar(c) => {
            state.input.insert(state.cursor_pos, *c);
            state.cursor_pos += c.len_utf8();
        }
        InputAction::DeleteChar => {
            if state.cursor_pos > 0 {
                let cursor_char_len = state.input[..state.cursor_pos]
                    .chars()
                    .rev()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(1);
                state.cursor_pos -= cursor_char_len;
                state.input.remove(state.cursor_pos);
            }
        }
        InputAction::DeleteCharForward => {
            if state.cursor_pos < state.input.len() {
                state.input.remove(state.cursor_pos);
            }
        }
        InputAction::DeleteWordBackward => {
            if state.cursor_pos > 0 {
                let chars: Vec<char> = state.input.chars().collect();
                let char_pos = state.input[..state.cursor_pos].chars().count();
                let mut pos = char_pos.saturating_sub(1);

                // Skip whitespace
                while pos > 0 && chars.get(pos).map(|c| c.is_whitespace()).unwrap_or(false) {
                    pos -= 1;
                }

                // Skip word characters
                while pos > 0 && chars.get(pos).map(|c| !c.is_whitespace()).unwrap_or(false) {
                    pos -= 1;
                }

                if pos > 0 {
                    pos += 1;
                }

                let new_byte_pos: usize = chars[..pos].iter().map(|c| c.len_utf8()).sum();
                state.input.replace_range(new_byte_pos..state.cursor_pos, "");
                state.cursor_pos = new_byte_pos;
            }
        }
        InputAction::DeleteWordForward => {
            let chars: Vec<char> = state.input.chars().collect();
            let char_pos = state.input[..state.cursor_pos].chars().count();
            let mut pos = char_pos;

            // Skip word characters
            while pos < chars.len() && chars.get(pos).map(|c| !c.is_whitespace()).unwrap_or(false) {
                pos += 1;
            }

            // Skip whitespace
            while pos < chars.len() && chars.get(pos).map(|c| c.is_whitespace()).unwrap_or(false) {
                pos += 1;
            }

            let end_byte_pos: usize = chars[..pos].iter().map(|c| c.len_utf8()).sum();
            state.input.replace_range(state.cursor_pos..end_byte_pos, "");
        }
        InputAction::MoveCursorLeft => {
            if state.cursor_pos > 0 {
                let char_len = state.input[..state.cursor_pos]
                    .chars()
                    .rev()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(1);
                state.cursor_pos -= char_len;
            }
        }
        InputAction::MoveCursorRight => {
            if state.cursor_pos < state.input.len() {
                let char_len = state.input[state.cursor_pos..]
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(1);
                state.cursor_pos += char_len;
            }
        }
        InputAction::MoveCursorStart => {
            state.cursor_pos = 0;
        }
        InputAction::MoveCursorEnd => {
            state.cursor_pos = state.input.len();
        }
        InputAction::MoveCursorWordLeft => {
            let chars: Vec<char> = state.input.chars().collect();
            let char_pos = state.input[..state.cursor_pos].chars().count();

            if char_pos == 0 {
                return state;
            }

            let mut pos = char_pos.saturating_sub(1);

            // Skip current whitespace
            while pos > 0 && chars.get(pos).map(|c| c.is_whitespace()).unwrap_or(false) {
                pos -= 1;
            }

            // Skip word characters
            while pos > 0 && chars.get(pos).map(|c| !c.is_whitespace()).unwrap_or(false) {
                pos -= 1;
            }

            // Convert back to byte position
            state.cursor_pos = chars[..pos].iter().map(|c| c.len_utf8()).sum();
            if pos > 0 {
                state.cursor_pos += chars[pos].len_utf8();
            }
        }
        InputAction::MoveCursorWordRight => {
            let chars: Vec<char> = state.input.chars().collect();
            let char_pos = state.input[..state.cursor_pos].chars().count();

            if char_pos >= chars.len() {
                return state;
            }

            let mut pos = char_pos;

            // Skip current word
            while pos < chars.len() && chars.get(pos).map(|c| !c.is_whitespace()).unwrap_or(false) {
                pos += 1;
            }

            // Skip whitespace
            while pos < chars.len() && chars.get(pos).map(|c| c.is_whitespace()).unwrap_or(false) {
                pos += 1;
            }

            // Convert back to byte position
            state.cursor_pos = chars[..pos].iter().map(|c| c.len_utf8()).sum();
        }
        InputAction::ClearInput => {
            state.input.clear();
            state.cursor_pos = 0;
        }
        InputAction::SetInput(content) => {
            state.input = content.clone();
            state.cursor_pos = state.input.len();
        }
        InputAction::SubmitInput => {
            if !state.is_input_empty() {
                state.input_history.push(state.input.clone());
                state.input_history_index = None;
                state.saved_input.clear();
            }
        }
        InputAction::HistoryUp => {
            if state.input_history.is_empty() {
                return state;
            }

            if state.input_history_index.is_none() {
                state.saved_input = state.input.clone();
            }

            let history_len = state.input_history.len();
            let current_index = state.input_history_index.unwrap_or(history_len);

            if current_index > 0 {
                let new_index = current_index - 1;
                state.input_history_index = Some(new_index);
                state.input = state.input_history[new_index].clone();
                state.cursor_pos = state.input.len();
            }
        }
        InputAction::HistoryDown => {
            match state.input_history_index {
                None => {}
                Some(index) => {
                    if index + 1 >= state.input_history.len() {
                        state.input_history_index = None;
                        state.input = state.saved_input.clone();
                        state.cursor_pos = state.input.len();
                    } else {
                        let new_index = index + 1;
                        state.input_history_index = Some(new_index);
                        state.input = state.input_history[new_index].clone();
                        state.cursor_pos = state.input.len();
                    }
                }
            }
        }
    }
    state
}

/// Reduce navigation-related actions
fn reduce_navigation(mut state: State, action: &NavigationAction) -> State {
    match action {
        NavigationAction::ScrollUp(n) => {
            state.scroll_offset = state.scroll_offset.saturating_sub(*n);
        }
        NavigationAction::ScrollDown(n) => {
            let max_scroll = state.messages.len().saturating_sub(1);
            state.scroll_offset = (state.scroll_offset + n).min(max_scroll);
        }
        NavigationAction::ScrollToTop => {
            state.scroll_offset = 0;
        }
        NavigationAction::ScrollToBottom => {
            state.scroll_offset = state.messages.len().saturating_sub(1);
        }
        NavigationAction::ScrollToMessage(index) => {
            if *index < state.messages.len() {
                state.scroll_offset = *index;
            }
        }
        NavigationAction::ScrollToMessageById(id) => {
            if let Some((index, _)) = state.find_message_by_id(id) {
                state.scroll_offset = index;
            }
        }
        NavigationAction::NavigateBack => {
            state.mode = Mode::Normal;
        }
    }
    state
}

/// Reduce UI-related actions
fn reduce_ui(mut state: State, action: &UiAction) -> State {
    match action {
        UiAction::ToggleTheme => {
            state.theme = match state.theme {
                Theme::Light => Theme::Dark,
                Theme::Dark => Theme::Light,
            };
        }
        UiAction::SetTheme(dark) => {
            state.theme = if *dark { Theme::Dark } else { Theme::Light };
        }
        UiAction::SetAgentState(agent_state) => {
            state.agent_state = *agent_state;
        }
        UiAction::SetMode(mode) => {
            state.mode = *mode;
        }
        UiAction::Quit => {
            state.should_quit = true;
        }
        UiAction::ShowHelp => {
            state.show_help = true;
            state.focus = FocusTarget::HelpOverlay;
        }
        UiAction::HideHelp => {
            state.show_help = false;
            state.focus = FocusTarget::Input;
        }
        UiAction::ToggleHelp => {
            state.show_help = !state.show_help;
            state.focus = if state.show_help {
                FocusTarget::HelpOverlay
            } else {
                FocusTarget::Input
            };
        }
        UiAction::SetFocus(target) => {
            state.focus = *target;
        }
        UiAction::ShowStatus { message, is_error } => {
            state.status_message = Some(message.clone());
            state.status_is_error = *is_error;
        }
        UiAction::ClearStatus => {
            state.status_message = None;
            state.status_is_error = false;
        }
    }
    state
}

/// Reduce search-related actions
fn reduce_search(mut state: State, action: &SearchAction) -> State {
    match action {
        SearchAction::Search(query) => {
            if query.is_empty() {
                state.search_query = None;
                state.search_results.clear();
                state.current_match_index = None;
            } else {
                state.search_query = Some(query.clone());
                state.search_results = search_messages(&state.messages, query, state.case_sensitive);
                state.current_match_index = if state.search_results.is_empty() {
                    None
                } else {
                    Some(0)
                };

                // Scroll to first match
                if let Some(first_match) = state.search_results.first() {
                    state.scroll_offset = first_match.message_index;
                }
            }
        }
        SearchAction::ClearSearch => {
            state.search_query = None;
            state.search_results.clear();
            state.current_match_index = None;
        }
        SearchAction::NextMatch => {
            if !state.search_results.is_empty() {
                let current = state.current_match_index.unwrap_or(0);
                let next = (current + 1) % state.search_results.len();
                state.current_match_index = Some(next);

                // Scroll to match
                if let Some(match_) = state.search_results.get(next) {
                    state.scroll_offset = match_.message_index;
                }
            }
        }
        SearchAction::PrevMatch => {
            if !state.search_results.is_empty() {
                let current = state.current_match_index.unwrap_or(0);
                let prev = if current == 0 {
                    state.search_results.len() - 1
                } else {
                    current - 1
                };
                state.current_match_index = Some(prev);

                // Scroll to match
                if let Some(match_) = state.search_results.get(prev) {
                    state.scroll_offset = match_.message_index;
                }
            }
        }
        SearchAction::ToggleCaseSensitive => {
            state.case_sensitive = !state.case_sensitive;
            // Re-run search with new setting
            if let Some(query) = &state.search_query {
                let query = query.clone();
                state.search_results = search_messages(&state.messages, &query, state.case_sensitive);
                state.current_match_index = if state.search_results.is_empty() {
                    None
                } else {
                    Some(0)
                };
            }
        }
    }
    state
}

/// Reduce session-related actions
fn reduce_session(mut state: State, action: &SessionAction) -> State {
    match action {
        SessionAction::NewSession => {
            let session = Session::new();
            state.sessions.insert(session.id.clone(), session.clone());
            state.current_session = session;
            state.sync_messages();
            state.scroll_offset = 0;
            state.search_query = None;
            state.search_results.clear();
            state.current_match_index = None;
            state.status_message = Some("New session created".to_string());
            state.status_is_error = false;
        }
        SessionAction::NewSessionWithName(name) => {
            let session = Session::with_name(name);
            state.sessions.insert(session.id.clone(), session.clone());
            state.current_session = session;
            state.sync_messages();
            state.scroll_offset = 0;
            state.status_message = Some("New session created".to_string());
            state.status_is_error = false;
        }
        SessionAction::SwitchSession(id) => {
            if let Some(session) = state.sessions.get(id).cloned() {
                // Save current session first
                state.sessions.insert(
                    state.current_session.id.clone(),
                    state.current_session.clone(),
                );

                state.current_session = session;
                state.sync_messages();
                state.scroll_offset = state.messages.len().saturating_sub(1);
                state.search_query = None;
                state.search_results.clear();
                state.current_match_index = None;
            }
        }
        SessionAction::DeleteSession(id) => {
            // Don't delete if it's the only session
            if state.sessions.len() <= 1 {
                state.status_message = Some("Cannot delete the last session".to_string());
                state.status_is_error = true;
                return state;
            }

            state.sessions.remove(id);

            // If we deleted the current session, switch to another
            if state.current_session.id == *id {
                if let Some(session) = state.sessions.values().next().cloned() {
                    state.current_session = session;
                    state.sync_messages();
                    state.scroll_offset = 0;
                }
            }
        }
        SessionAction::RenameSession(name) => {
            state.current_session.name = Some(name.clone());
            state.current_session.touch();
        }
        SessionAction::SaveSession => {
            // In a real implementation, this would persist to disk
            state.sessions.insert(
                state.current_session.id.clone(),
                state.current_session.clone(),
            );
            state.status_message = Some("Session saved".to_string());
            state.status_is_error = false;
        }
        SessionAction::LoadSession(id) => {
            if let Some(session) = state.sessions.get(id).cloned() {
                state.current_session = session;
                state.sync_messages();
                state.scroll_offset = state.messages.len().saturating_sub(1);
            }
        }
        SessionAction::ListSessions => {
            // This could set a flag to show session list in UI
            // For now, just set a status message
            let count = state.sessions.len();
            state.status_message = Some(format!("{} sessions available", count));
            state.status_is_error = false;
        }
    }
    state
}

/// Reduce command-related actions
fn reduce_command(mut state: State, action: &CommandAction) -> State {
    match action {
        CommandAction::AddCommand(cmd) => {
            // Don't add duplicate consecutive commands
            if state.command_history.last() != Some(cmd) {
                state.command_history.push(cmd.clone());
                // Keep only last 100 commands
                if state.command_history.len() > 100 {
                    state.command_history.remove(0);
                }
            }
        }
        CommandAction::CommandHistoryUp => {
            if state.command_history.is_empty() {
                return state;
            }

            if state.command_history_index.is_none() {
                state.saved_command = state.input.clone();
            }

            let history_len = state.command_history.len();
            let current_index = state.command_history_index.unwrap_or(history_len);

            if current_index > 0 {
                let new_index = current_index - 1;
                state.command_history_index = Some(new_index);
                state.input = state.command_history[new_index].clone();
                state.cursor_pos = state.input.len();
            }
        }
        CommandAction::CommandHistoryDown => {
            match state.command_history_index {
                None => {}
                Some(index) => {
                    if index + 1 >= state.command_history.len() {
                        state.command_history_index = None;
                        state.input = state.saved_command.clone();
                        state.cursor_pos = state.input.len();
                    } else {
                        let new_index = index + 1;
                        state.command_history_index = Some(new_index);
                        state.input = state.command_history[new_index].clone();
                        state.cursor_pos = state.input.len();
                    }
                }
            }
        }
        CommandAction::ClearCommandHistory => {
            state.command_history.clear();
            state.command_history_index = None;
            state.saved_command.clear();
        }
    }
    state
}

/// Search messages for a query
fn search_messages(messages: &[Message], query: &str, case_sensitive: bool) -> Vec<SearchMatch> {
    let search_query = if case_sensitive {
        query.to_string()
    } else {
        query.to_lowercase()
    };

    let mut results = Vec::new();

    for (msg_idx, msg) in messages.iter().enumerate() {
        let content = if case_sensitive {
            msg.content.clone()
        } else {
            msg.content.to_lowercase()
        };

        let mut start = 0;
        while let Some(pos) = content[start..].find(&search_query) {
            let absolute_pos = start + pos;
            results.push(SearchMatch {
                message_index: msg_idx,
                start: absolute_pos,
                end: absolute_pos + search_query.len(),
            });
            start = absolute_pos + search_query.len();
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_default() {
        let state = State::new();
        assert!(state.messages.is_empty());
        assert!(state.input.is_empty());
        assert_eq!(state.cursor_pos, 0);
        assert_eq!(state.mode, Mode::Input);
        assert!(!state.should_quit);
    }

    #[test]
    fn test_add_message() {
        let state = State::new();
        let action = Action::user_message("Hello");
        let new_state = reduce(state, &action);

        assert_eq!(new_state.messages.len(), 1);
        assert_eq!(new_state.messages[0].content, "Hello");
    }

    #[test]
    fn test_input_navigation() {
        let state = State::new();
        let actions = vec![
            Action::insert_char('a'),
            Action::insert_char('b'),
            Action::insert_char('c'),
        ];
        let state = actions.iter().fold(state, |s, a| reduce(s, a));

        assert_eq!(state.input, "abc");
        assert_eq!(state.cursor_pos, 3);

        let state = reduce(state, &Action::Input(InputAction::MoveCursorLeft));
        assert_eq!(state.cursor_pos, 2);

        let state = reduce(state, &Action::delete_char());
        assert_eq!(state.input, "ac");
        assert_eq!(state.cursor_pos, 1);
    }

    #[test]
    fn test_search() {
        let state = State::new();
        let state = reduce(state, &Action::user_message("Hello World"));
        let state = reduce(state, &Action::user_message("Hello Rust"));
        let state = reduce(state, &Action::assistant_message("Hello AI"));

        let state = reduce(state, &Action::search("Hello"));
        assert_eq!(state.search_results.len(), 3);
        assert_eq!(state.current_match_index, Some(0));

        let state = reduce(state, &Action::next_match());
        assert_eq!(state.current_match_index, Some(1));

        let state = reduce(state, &Action::prev_match());
        assert_eq!(state.current_match_index, Some(0));
    }

    #[test]
    fn test_session_management() {
        let state = State::new();
        let session_id = state.current_session.id.clone();

        let state = reduce(state, &Action::user_message("Test"));
        assert_eq!(state.messages.len(), 1);

        let state = reduce(state, &Action::Session(SessionAction::NewSession));
        assert_ne!(state.current_session.id, session_id);
        assert!(state.messages.is_empty());
    }

    #[test]
    fn test_command_history() {
        let state = State::new();

        let state = reduce(state, &Action::Command(CommandAction::AddCommand("q".to_string())));
        let state = reduce(state, &Action::Command(CommandAction::AddCommand("help".to_string())));

        assert_eq!(state.command_history.len(), 2);

        let state = reduce(state, &Action::Command(CommandAction::CommandHistoryUp));
        assert_eq!(state.input, "help");
    }

    #[test]
    fn test_delete_word_backward() {
        let state = State::new();
        let state = reduce(state, &Action::Input(InputAction::SetInput("hello world test".to_string())));

        // Move cursor to end
        let state = reduce(state, &Action::Input(InputAction::MoveCursorEnd));

        // Delete "test"
        let state = reduce(state, &Action::Input(InputAction::DeleteWordBackward));
        assert_eq!(state.input, "hello world ");
    }

    #[test]
    fn test_theme_toggle() {
        let state = State::new();
        assert_eq!(state.theme, Theme::Light);

        let state = reduce(state, &Action::toggle_theme());
        assert_eq!(state.theme, Theme::Dark);

        let state = reduce(state, &Action::toggle_theme());
        assert_eq!(state.theme, Theme::Light);
    }

    #[test]
    fn test_mode_change() {
        let state = State::new();
        assert_eq!(state.mode, Mode::Input);

        let state = reduce(state, &Action::set_mode(Mode::Normal));
        assert_eq!(state.mode, Mode::Normal);

        let state = reduce(state, &Action::set_mode(Mode::Command));
        assert_eq!(state.mode, Mode::Command);
    }

    #[test]
    fn test_batch_actions() {
        let state = State::new();
        let action = Action::batch(vec![
            Action::insert_char('a'),
            Action::insert_char('b'),
            Action::insert_char('c'),
        ]);
        let state = reduce(state, &action);

        assert_eq!(state.input, "abc");
    }
}
