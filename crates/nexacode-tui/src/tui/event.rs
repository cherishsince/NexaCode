//! Event handling
//!
//! Converts keyboard/mouse events into Actions and dispatches them to the Store.

use crate::{Action, CommandAction, InputAction, Mode, SearchAction, SessionAction, Store};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::Duration;

pub async fn handle_event(store: &mut Store) -> anyhow::Result<bool> {
    if !event::poll(Duration::from_millis(100))? {
        return Ok(false);
    }

    match event::read()? {
        Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
            handle_key_event(store, key_event)
        }
        _ => Ok(false),
    }
}

fn handle_key_event(store: &mut Store, key_event: KeyEvent) -> anyhow::Result<bool> {
    let state = store.state();

    // Handle keys based on current mode
    match state.mode {
        Mode::Normal => handle_normal_mode(store, key_event),
        Mode::Input => handle_input_mode(store, key_event),
        Mode::Command => handle_command_mode(store, key_event),
        Mode::Search => handle_search_mode(store, key_event),
    }
}

/// Handle keys in Normal mode (navigation, mode switching)
fn handle_normal_mode(store: &mut Store, key_event: KeyEvent) -> anyhow::Result<bool> {
    match (key_event.modifiers, key_event.code) {
        // Ctrl+C quit
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            store.dispatch(Action::quit());
            Ok(true)
        }
        // q quit
        (KeyModifiers::NONE, KeyCode::Char('q')) => {
            store.dispatch(Action::quit());
            Ok(true)
        }
        // i or a enter input mode
        (KeyModifiers::NONE, KeyCode::Char('i')) | (KeyModifiers::NONE, KeyCode::Char('a')) => {
            store.dispatch(Action::set_mode(Mode::Input));
            Ok(true)
        }
        // : enter command mode
        (KeyModifiers::NONE, KeyCode::Char(':')) => {
            store.dispatch(Action::set_mode(Mode::Command));
            Ok(true)
        }
        // / enter search mode
        (KeyModifiers::NONE, KeyCode::Char('/')) => {
            store.dispatch(Action::set_mode(Mode::Search));
            Ok(true)
        }
        // t toggle theme
        (KeyModifiers::NONE, KeyCode::Char('t')) | (KeyModifiers::CONTROL, KeyCode::Char('t')) => {
            store.dispatch(Action::toggle_theme());
            Ok(true)
        }
        // j or Down scroll down
        (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
            store.dispatch(Action::scroll_down(1));
            Ok(true)
        }
        // k or Up scroll up
        (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
            store.dispatch(Action::scroll_up(1));
            Ok(true)
        }
        // g scroll to top
        (KeyModifiers::NONE, KeyCode::Char('g')) => {
            store.dispatch(Action::scroll_to_top());
            Ok(true)
        }
        // G scroll to bottom
        (KeyModifiers::NONE, KeyCode::Char('G')) => {
            store.dispatch(Action::scroll_to_bottom());
            Ok(true)
        }
        // u undo
        (KeyModifiers::NONE, KeyCode::Char('u')) => {
            store.dispatch(Action::Undo);
            Ok(true)
        }
        // Ctrl+R redo
        (KeyModifiers::CONTROL, KeyCode::Char('r')) => {
            store.dispatch(Action::Redo);
            Ok(true)
        }
        // ? show help
        (KeyModifiers::SHIFT, KeyCode::Char('?')) => {
            store.dispatch(Action::toggle_help());
            Ok(true)
        }
        // h show help
        (KeyModifiers::NONE, KeyCode::Char('h')) => {
            store.dispatch(Action::toggle_help());
            Ok(true)
        }
        // n next search match
        (KeyModifiers::NONE, KeyCode::Char('n')) => {
            store.dispatch(Action::next_match());
            Ok(true)
        }
        // N previous search match
        (KeyModifiers::SHIFT, KeyCode::Char('N')) => {
            store.dispatch(Action::prev_match());
            Ok(true)
        }
        // Space or Enter enter input mode
        (KeyModifiers::NONE, KeyCode::Char(' ')) | (KeyModifiers::NONE, KeyCode::Enter) => {
            store.dispatch(Action::set_mode(Mode::Input));
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Handle keys in Input mode (text input, editing)
fn handle_input_mode(store: &mut Store, key_event: KeyEvent) -> anyhow::Result<bool> {
    match (key_event.modifiers, key_event.code) {
        // Ctrl+C quit
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            store.dispatch(Action::quit());
            Ok(true)
        }
        // Escape return to normal mode
        (KeyModifiers::NONE, KeyCode::Esc) => {
            store.dispatch(Action::set_mode(Mode::Normal));
            Ok(true)
        }
        // Enter submit input
        (KeyModifiers::NONE, KeyCode::Enter) => {
            let content = store.state().input.clone();
            if !content.trim().is_empty() {
                // Add user message
                store.dispatch(Action::user_message(content));

                // TODO: Here we will connect to LLM for processing
                // Simulated AI response
                store.dispatch(Action::assistant_message(
                    "I received your message. This is a placeholder response.\n\
                     In the future, this will be connected to an LLM."
                ));
            }
            Ok(true)
        }
        // Ctrl+T toggle theme
        (KeyModifiers::CONTROL, KeyCode::Char('t')) => {
            store.dispatch(Action::toggle_theme());
            Ok(true)
        }
        // Character input
        (KeyModifiers::NONE, KeyCode::Char(c)) => {
            store.dispatch(Action::insert_char(c));
            Ok(true)
        }
        // Ctrl+A move cursor to start
        (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
            store.dispatch(Action::Input(InputAction::MoveCursorStart));
            Ok(true)
        }
        // Ctrl+E move cursor to end
        (KeyModifiers::CONTROL, KeyCode::Char('e')) => {
            store.dispatch(Action::Input(InputAction::MoveCursorEnd));
            Ok(true)
        }
        // Ctrl+W delete word backward
        (KeyModifiers::CONTROL, KeyCode::Char('w')) => {
            store.dispatch(Action::Input(InputAction::DeleteWordBackward));
            Ok(true)
        }
        // Ctrl+U clear input
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            store.dispatch(Action::Input(InputAction::ClearInput));
            Ok(true)
        }
        // Ctrl+K delete word forward / to end
        (KeyModifiers::CONTROL, KeyCode::Char('k')) => {
            store.dispatch(Action::Input(InputAction::DeleteWordForward));
            Ok(true)
        }
        // Backspace delete
        (KeyModifiers::NONE, KeyCode::Backspace) => {
            store.dispatch(Action::delete_char());
            Ok(true)
        }
        // Delete delete forward
        (KeyModifiers::NONE, KeyCode::Delete) => {
            store.dispatch(Action::Input(InputAction::DeleteCharForward));
            Ok(true)
        }
        // Left move cursor left
        (KeyModifiers::NONE, KeyCode::Left) => {
            store.dispatch(Action::Input(InputAction::MoveCursorLeft));
            Ok(true)
        }
        // Right move cursor right
        (KeyModifiers::NONE, KeyCode::Right) => {
            store.dispatch(Action::Input(InputAction::MoveCursorRight));
            Ok(true)
        }
        // Home cursor to start
        (KeyModifiers::NONE, KeyCode::Home) => {
            store.dispatch(Action::Input(InputAction::MoveCursorStart));
            Ok(true)
        }
        // End cursor to end
        (KeyModifiers::NONE, KeyCode::End) => {
            store.dispatch(Action::Input(InputAction::MoveCursorEnd));
            Ok(true)
        }
        // Up navigate input history or scroll
        (KeyModifiers::NONE, KeyCode::Up) => {
            if store.state().input.is_empty() {
                store.dispatch(Action::scroll_up(1));
            } else {
                store.dispatch(Action::Input(InputAction::HistoryUp));
            }
            Ok(true)
        }
        // Down navigate input history or scroll
        (KeyModifiers::NONE, KeyCode::Down) => {
            if store.state().input.is_empty() {
                store.dispatch(Action::scroll_down(1));
            } else {
                store.dispatch(Action::Input(InputAction::HistoryDown));
            }
            Ok(true)
        }
        // Ctrl+Left move word left
        (KeyModifiers::CONTROL, KeyCode::Left) => {
            store.dispatch(Action::Input(InputAction::MoveCursorWordLeft));
            Ok(true)
        }
        // Alt+Left move word left (alternative)
        (KeyModifiers::ALT, KeyCode::Left) => {
            store.dispatch(Action::Input(InputAction::MoveCursorWordLeft));
            Ok(true)
        }
        // Ctrl+Right move word right
        (KeyModifiers::CONTROL, KeyCode::Right) => {
            store.dispatch(Action::Input(InputAction::MoveCursorWordRight));
            Ok(true)
        }
        // Alt+Right move word right (alternative)
        (KeyModifiers::ALT, KeyCode::Right) => {
            store.dispatch(Action::Input(InputAction::MoveCursorWordRight));
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Handle keys in Command mode (commands like :quit, :help)
fn handle_command_mode(store: &mut Store, key_event: KeyEvent) -> anyhow::Result<bool> {
    match (key_event.modifiers, key_event.code) {
        // Escape return to normal mode
        (KeyModifiers::NONE, KeyCode::Esc) => {
            store.dispatch(Action::Input(InputAction::ClearInput));
            store.dispatch(Action::set_mode(Mode::Normal));
            Ok(true)
        }
        // Enter execute command
        (KeyModifiers::NONE, KeyCode::Enter) => {
            let command = store.state().input.trim().to_string();
            execute_command(store, &command);
            Ok(true)
        }
        // Character input for command
        (KeyModifiers::NONE, KeyCode::Char(c)) => {
            store.dispatch(Action::insert_char(c));
            Ok(true)
        }
        // Backspace
        (KeyModifiers::NONE, KeyCode::Backspace) => {
            store.dispatch(Action::delete_char());
            if store.state().input.is_empty() {
                store.dispatch(Action::set_mode(Mode::Normal));
            }
            Ok(true)
        }
        // Up navigate command history
        (KeyModifiers::NONE, KeyCode::Up) => {
            store.dispatch(Action::Command(CommandAction::CommandHistoryUp));
            Ok(true)
        }
        // Down navigate command history
        (KeyModifiers::NONE, KeyCode::Down) => {
            store.dispatch(Action::Command(CommandAction::CommandHistoryDown));
            Ok(true)
        }
        // Tab for command completion (future feature)
        (KeyModifiers::NONE, KeyCode::Tab) => {
            // TODO: Implement command completion
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Handle keys in Search mode
fn handle_search_mode(store: &mut Store, key_event: KeyEvent) -> anyhow::Result<bool> {
    match (key_event.modifiers, key_event.code) {
        // Escape return to normal mode
        (KeyModifiers::NONE, KeyCode::Esc) => {
            store.dispatch(Action::Input(InputAction::ClearInput));
            store.dispatch(Action::clear_search());
            store.dispatch(Action::set_mode(Mode::Normal));
            Ok(true)
        }
        // Enter confirm search
        (KeyModifiers::NONE, KeyCode::Enter) => {
            let query = store.state().input.trim().to_string();
            if !query.is_empty() {
                store.dispatch(Action::search(&query));
            }
            store.dispatch(Action::Input(InputAction::ClearInput));
            store.dispatch(Action::set_mode(Mode::Normal));
            Ok(true)
        }
        // Character input for search
        (KeyModifiers::NONE, KeyCode::Char(c)) => {
            store.dispatch(Action::insert_char(c));
            // Live search as you type
            let query = store.state().input.clone();
            if !query.is_empty() {
                store.dispatch(Action::Search(SearchAction::Search(query)));
            }
            Ok(true)
        }
        // Backspace
        (KeyModifiers::NONE, KeyCode::Backspace) => {
            store.dispatch(Action::delete_char());
            let query = store.state().input.clone();
            if query.is_empty() {
                store.dispatch(Action::clear_search());
                store.dispatch(Action::set_mode(Mode::Normal));
            } else {
                // Live search update
                store.dispatch(Action::Search(SearchAction::Search(query)));
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Execute a command string
fn execute_command(store: &mut Store, command: &str) {
    // Add to command history first
    store.dispatch(Action::Command(CommandAction::AddCommand(command.to_string())));

    // Parse command and arguments
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }

    let cmd = parts[0];
    let args = &parts[1..];

    match cmd {
        // Quit commands
        "q" | "quit" | "exit" => {
            store.dispatch(Action::quit());
        }
        // Help commands
        "h" | "help" | "?" => {
            store.dispatch(Action::show_help());
        }
        // Clear messages
        "clear" | "cls" => {
            store.dispatch(Action::clear_messages());
            store.dispatch(Action::show_status("Messages cleared", false));
        }
        // Theme toggle
        "theme" | "th" => {
            store.dispatch(Action::toggle_theme());
        }
        // New session
        "new" | "newsession" => {
            if args.is_empty() {
                store.dispatch(Action::new_session());
            } else {
                store.dispatch(Action::new_session_with_name(args.join(" ")));
            }
        }
        // Save session
        "save" | "w" => {
            store.dispatch(Action::save_session());
        }
        // List sessions
        "sessions" | "ls" => {
            store.dispatch(Action::Session(SessionAction::ListSessions));
        }
        // Search
        "search" | "find" | "/" => {
            if args.is_empty() {
                store.dispatch(Action::show_status("Usage: :search <query>", true));
            } else {
                store.dispatch(Action::search(args.join(" ")));
                store.dispatch(Action::set_mode(Mode::Normal));
            }
        }
        // Undo
        "undo" | "u" => {
            store.dispatch(Action::Undo);
        }
        // Redo
        "redo" | "r" => {
            store.dispatch(Action::Redo);
        }
        // Toggle case sensitive search
        "case" | "ic" => {
            store.dispatch(Action::Search(SearchAction::ToggleCaseSensitive));
            let state = store.state();
            let status = if state.case_sensitive {
                "Case sensitive search enabled"
            } else {
                "Case insensitive search enabled"
            };
            store.dispatch(Action::show_status(status, false));
        }
        // Unknown command
        _ => {
            store.dispatch(Action::show_status(format!("Unknown command: {}", cmd), true));
        }
    }

    // Clear input and return to normal mode
    store.dispatch(Action::Input(InputAction::ClearInput));
    store.dispatch(Action::set_mode(Mode::Normal));
}
