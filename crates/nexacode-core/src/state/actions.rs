//! Action definitions
//!
//! Actions are the only way to modify state. They represent
//! all possible state mutations in the application.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Timestamp type for messages
pub type Timestamp = u64;

/// Get current timestamp in milliseconds
pub fn now_timestamp() -> Timestamp {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Message role in a conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

impl Default for MessageRole {
    fn default() -> Self {
        Self::User
    }
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "User"),
            Self::Assistant => write!(f, "Assistant"),
            Self::System => write!(f, "System"),
            Self::Tool => write!(f, "Tool"),
        }
    }
}

/// Message content with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique message ID
    pub id: String,
    /// Role of the message sender
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// Timestamp in milliseconds since Unix epoch
    pub timestamp: Timestamp,
    /// Optional metadata for tool calls, attachments, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl Message {
    /// Create a new message with auto-generated ID and timestamp
    pub fn new(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            id: crate::state::reducers::generate_message_id(),
            role,
            content: content.into(),
            timestamp: now_timestamp(),
            metadata: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(MessageRole::User, content)
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Assistant, content)
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::new(MessageRole::System, content)
    }

    pub fn tool(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Tool, content)
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_timestamp(mut self, timestamp: Timestamp) -> Self {
        self.timestamp = timestamp;
        self
    }
}

/// Agent operational state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentState {
    #[default]
    Idle,
    Thinking,
    ExecutingTool,
    StreamingResponse,
    Error,
}

/// Application mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    Normal,
    #[default]
    Input,
    Command,
    Search,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal => write!(f, "NORMAL"),
            Self::Input => write!(f, "INPUT"),
            Self::Command => write!(f, "COMMAND"),
            Self::Search => write!(f, "SEARCH"),
        }
    }
}

// ============================================================================
// Action Definitions
// ============================================================================

/// Actions related to messages
#[derive(Debug, Clone)]
pub enum MessageAction {
    /// Add a new message
    AddMessage(Message),
    /// Clear all messages
    ClearMessages,
    /// Delete a specific message by index
    DeleteMessage(usize),
    /// Delete a message by ID
    DeleteMessageById(String),
    /// Edit a message's content
    EditMessage { index: usize, content: String },
    /// Edit a message by ID
    EditMessageById { id: String, content: String },
}

/// Actions related to input handling
#[derive(Debug, Clone)]
pub enum InputAction {
    /// Insert a character at cursor position
    InsertChar(char),
    /// Delete character before cursor
    DeleteChar,
    /// Delete character at cursor position
    DeleteCharForward,
    /// Delete word before cursor
    DeleteWordBackward,
    /// Delete word after cursor
    DeleteWordForward,
    /// Move cursor left
    MoveCursorLeft,
    /// Move cursor right
    MoveCursorRight,
    /// Move cursor to start of line
    MoveCursorStart,
    /// Move cursor to end of line
    MoveCursorEnd,
    /// Move cursor by word left
    MoveCursorWordLeft,
    /// Move cursor by word right
    MoveCursorWordRight,
    /// Clear input
    ClearInput,
    /// Set input content directly
    SetInput(String),
    /// Submit current input
    SubmitInput,
    /// Navigate input history
    HistoryUp,
    HistoryDown,
}

/// Actions related to navigation
#[derive(Debug, Clone)]
pub enum NavigationAction {
    /// Scroll up by N lines
    ScrollUp(usize),
    /// Scroll down by N lines
    ScrollDown(usize),
    /// Scroll to top
    ScrollToTop,
    /// Scroll to bottom
    ScrollToBottom,
    /// Scroll to specific message
    ScrollToMessage(usize),
    /// Scroll to specific message by ID
    ScrollToMessageById(String),
    /// Navigate to previous view
    NavigateBack,
}

/// Actions related to UI state
#[derive(Debug, Clone)]
pub enum UiAction {
    /// Toggle theme
    ToggleTheme,
    /// Set specific theme (true = dark)
    SetTheme(bool),
    /// Set agent state
    SetAgentState(AgentState),
    /// Set application mode
    SetMode(Mode),
    /// Quit application
    Quit,
    /// Show help
    ShowHelp,
    /// Hide help
    HideHelp,
    /// Toggle help
    ToggleHelp,
    /// Set focus to specific element
    SetFocus(FocusTarget),
    /// Show status message
    ShowStatus { message: String, is_error: bool },
    /// Clear status message
    ClearStatus,
}

/// Focus targets in the UI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusTarget {
    #[default]
    Input,
    MessageList,
    Sidebar,
    HelpOverlay,
}

/// Actions related to search
#[derive(Debug, Clone)]
pub enum SearchAction {
    /// Start a new search with the given query
    Search(String),
    /// Clear search results
    ClearSearch,
    /// Jump to next match
    NextMatch,
    /// Jump to previous match
    PrevMatch,
    /// Toggle case sensitivity
    ToggleCaseSensitive,
}

/// Actions related to session management
#[derive(Debug, Clone)]
pub enum SessionAction {
    /// Create a new session
    NewSession,
    /// Create a new session with a name
    NewSessionWithName(String),
    /// Switch to a session by ID
    SwitchSession(String),
    /// Delete a session by ID
    DeleteSession(String),
    /// Rename current session
    RenameSession(String),
    /// Save current session
    SaveSession,
    /// Load a session
    LoadSession(String),
    /// List all sessions
    ListSessions,
}

/// Actions related to command history
#[derive(Debug, Clone)]
pub enum CommandAction {
    /// Add command to history
    AddCommand(String),
    /// Navigate command history up
    CommandHistoryUp,
    /// Navigate command history down
    CommandHistoryDown,
    /// Clear command history
    ClearCommandHistory,
}

/// The root action enum that encompasses all state mutations
#[derive(Debug, Clone)]
pub enum Action {
    // Message actions
    Message(MessageAction),
    // Input actions
    Input(InputAction),
    // Navigation actions
    Navigation(NavigationAction),
    // UI actions
    Ui(UiAction),
    // Search actions
    Search(SearchAction),
    // Session actions
    Session(SessionAction),
    // Command actions
    Command(CommandAction),
    // Undo/Redo actions
    Undo,
    Redo,
    // Composite actions (batch multiple actions)
    Batch(Vec<Action>),
}

// Convenience impls for creating actions
impl Action {
    pub fn add_message(role: MessageRole, content: impl Into<String>) -> Self {
        Action::Message(MessageAction::AddMessage(Message::new(role, content)))
    }

    pub fn user_message(content: impl Into<String>) -> Self {
        Action::Message(MessageAction::AddMessage(Message::user(content)))
    }

    pub fn assistant_message(content: impl Into<String>) -> Self {
        Action::Message(MessageAction::AddMessage(Message::assistant(content)))
    }

    pub fn system_message(content: impl Into<String>) -> Self {
        Action::Message(MessageAction::AddMessage(Message::system(content)))
    }

    pub fn clear_messages() -> Self {
        Action::Message(MessageAction::ClearMessages)
    }

    pub fn delete_message(index: usize) -> Self {
        Action::Message(MessageAction::DeleteMessage(index))
    }

    pub fn submit_input() -> Self {
        Action::Input(InputAction::SubmitInput)
    }

    pub fn insert_char(c: char) -> Self {
        Action::Input(InputAction::InsertChar(c))
    }

    pub fn delete_char() -> Self {
        Action::Input(InputAction::DeleteChar)
    }

    pub fn set_mode(mode: Mode) -> Self {
        Action::Ui(UiAction::SetMode(mode))
    }

    pub fn set_agent_state(state: AgentState) -> Self {
        Action::Ui(UiAction::SetAgentState(state))
    }

    pub fn toggle_theme() -> Self {
        Action::Ui(UiAction::ToggleTheme)
    }

    pub fn quit() -> Self {
        Action::Ui(UiAction::Quit)
    }

    pub fn show_help() -> Self {
        Action::Ui(UiAction::ShowHelp)
    }

    pub fn hide_help() -> Self {
        Action::Ui(UiAction::HideHelp)
    }

    pub fn toggle_help() -> Self {
        Action::Ui(UiAction::ToggleHelp)
    }

    pub fn scroll_up(n: usize) -> Self {
        Action::Navigation(NavigationAction::ScrollUp(n))
    }

    pub fn scroll_down(n: usize) -> Self {
        Action::Navigation(NavigationAction::ScrollDown(n))
    }

    pub fn scroll_to_bottom() -> Self {
        Action::Navigation(NavigationAction::ScrollToBottom)
    }

    pub fn scroll_to_top() -> Self {
        Action::Navigation(NavigationAction::ScrollToTop)
    }

    pub fn search(query: impl Into<String>) -> Self {
        Action::Search(SearchAction::Search(query.into()))
    }

    pub fn clear_search() -> Self {
        Action::Search(SearchAction::ClearSearch)
    }

    pub fn next_match() -> Self {
        Action::Search(SearchAction::NextMatch)
    }

    pub fn prev_match() -> Self {
        Action::Search(SearchAction::PrevMatch)
    }

    pub fn new_session() -> Self {
        Action::Session(SessionAction::NewSession)
    }

    pub fn new_session_with_name(name: impl Into<String>) -> Self {
        Action::Session(SessionAction::NewSessionWithName(name.into()))
    }

    pub fn save_session() -> Self {
        Action::Session(SessionAction::SaveSession)
    }

    pub fn show_status(message: impl Into<String>, is_error: bool) -> Self {
        Action::Ui(UiAction::ShowStatus {
            message: message.into(),
            is_error,
        })
    }

    pub fn clear_status() -> Self {
        Action::Ui(UiAction::ClearStatus)
    }

    pub fn add_command(cmd: impl Into<String>) -> Self {
        Action::Command(CommandAction::AddCommand(cmd.into()))
    }

    pub fn batch(actions: Vec<Action>) -> Self {
        Action::Batch(actions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "Hello");
        assert!(msg.timestamp > 0);
        assert!(msg.metadata.is_none());
        assert!(msg.id.starts_with("msg_"));
    }

    #[test]
    fn test_message_with_metadata() {
        let metadata = serde_json::json!({"tool": "read_file"});
        let msg = Message::tool("file contents")
            .with_metadata(metadata.clone());
        assert_eq!(msg.role, MessageRole::Tool);
        assert_eq!(msg.metadata, Some(metadata));
    }

    #[test]
    fn test_action_helpers() {
        let action = Action::user_message("test");
        match action {
            Action::Message(MessageAction::AddMessage(msg)) => {
                assert_eq!(msg.role, MessageRole::User);
                assert_eq!(msg.content, "test");
            }
            _ => panic!("Expected MessageAction::AddMessage"),
        }
    }

    #[test]
    fn test_mode_display() {
        assert_eq!(format!("{}", Mode::Normal), "NORMAL");
        assert_eq!(format!("{}", Mode::Input), "INPUT");
        assert_eq!(format!("{}", Mode::Command), "COMMAND");
        assert_eq!(format!("{}", Mode::Search), "SEARCH");
    }
}
