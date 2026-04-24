//! NexaCode TUI Library
//!
//! This crate contains the Terminal User Interface of NexaCode:
//! - TUI components and views
//! - Event handling
//! - Theme system
//!
//! State management is provided by nexacode-core

pub mod tui;

// Re-export from nexacode-core for convenience
pub use nexacode_core::{
    Action, AgentState, CommandAction, FocusTarget, InputAction, Message, MessageAction,
    MessageRole, Mode, NavigationAction, SearchAction, SearchMatch, Session, SessionAction, State,
    Store, Subscriber, SubscriberId, UiAction,
};

// Re-export TUI types
pub use self::tui::Theme;
pub use self::tui::event::handle_event;
pub use self::tui::views::render;
