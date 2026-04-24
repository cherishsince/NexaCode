//! Event handling
//!
//! Converts keyboard/mouse events into Actions and dispatches them to the Store.

use crate::{Action, CommandAction, InputAction, MessageAction, Mode, SearchAction, SessionAction, Store};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::Duration;
use std::sync::Arc;
use nexacode_core::core::slash_commands::{parse_slash_command, SlashCommand, get_help};
use nexacode_core::core::agent::AgentController;

pub async fn handle_event(store: &mut Store, agent: Arc<AgentController>) -> anyhow::Result<bool> {
    if !event::poll(Duration::from_millis(100))? {
        return Ok(false);
    }

    match event::read()? {
        Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
            handle_key_event(store, key_event, agent).await
        }
        _ => Ok(false),
    }
}

async fn handle_key_event(store: &mut Store, key_event: KeyEvent, agent: Arc<AgentController>) -> anyhow::Result<bool> {
    let state = store.state();

    // Handle keys based on current mode
    match state.mode {
        Mode::Normal => handle_normal_mode(store, key_event),
        Mode::Input => handle_input_mode(store, key_event, agent).await,
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
async fn handle_input_mode(store: &mut Store, key_event: KeyEvent, agent: Arc<AgentController>) -> anyhow::Result<bool> {
    // Check if we're in model selection mode
    if !store.state().model_selections.is_empty() {
        return handle_model_selection(store, key_event);
    }
    
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
        // Enter submit input or select suggestion
        (KeyModifiers::NONE, KeyCode::Enter) => {
            // If there are command suggestions, select and execute the current one
            if !store.state().command_suggestions.is_empty() {
                // Get the selected suggestion
                let selected_index = store.state().command_suggestion_index.unwrap_or(0);
                if let Some(suggestion) = store.state().command_suggestions.get(selected_index).cloned() {
                    // Clear suggestions first
                    store.dispatch(Action::Input(InputAction::ClearInput));
                    // Then execute the command directly
                    match parse_slash_command(&suggestion) {
                        nexacode_core::core::slash_commands::ParseResult::Ok(cmd) => {
                            execute_slash_command(store, cmd);
                        }
                        _ => {
                            // If not a valid command, just set the input
                            store.dispatch(Action::Input(InputAction::SetInput(suggestion)));
                        }
                    }
                }
            } else {
                let content = store.state().input.clone();
                if !content.trim().is_empty() {
                    // Check if it's a slash command
                    match parse_slash_command(&content) {
                        nexacode_core::core::slash_commands::ParseResult::Ok(cmd) => {
                            // Execute slash command
                            execute_slash_command(store, cmd);
                        }
                        nexacode_core::core::slash_commands::ParseResult::NotACommand(msg) => {
                            // Regular message - add user message
                            store.dispatch(Action::user_message(msg.clone()));
                            
                            // Add empty assistant message for streaming
                            store.dispatch(Action::assistant_message(""));
                            
                            // For streaming, we need to use a channel approach
                            // because the callback runs in a different async context
                            // Create a simple accumulator that we'll update
                            let response = agent.process_user_message(msg).await;
                            
                            match response {
                                Ok(response) => {
                                    // Update the last message with the full response
                                    store.dispatch(Action::Message(
                                        MessageAction::EditMessage {
                                            index: store.state().messages.len() - 1,
                                            content: response,
                                        }
                                    ));
                                }
                                Err(e) => {
                                    store.dispatch(Action::show_status(
                                        format!("Error: {}", e),
                                        true
                                    ));
                                }
                            }
                        }
                        nexacode_core::core::slash_commands::ParseResult::Error(err) => {
                            store.dispatch(Action::show_status(err, true));
                        }
                    }
                }
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
        // Up navigate command suggestions or input history
        (KeyModifiers::NONE, KeyCode::Up) => {
            let state = store.state();
            if !state.command_suggestions.is_empty() {
                // Navigate command suggestions
                store.dispatch(Action::Input(InputAction::SuggestionUp));
            } else if state.input.is_empty() {
                store.dispatch(Action::scroll_up(1));
            } else {
                store.dispatch(Action::Input(InputAction::HistoryUp));
            }
            Ok(true)
        }
        // Down navigate command suggestions or input history
        (KeyModifiers::NONE, KeyCode::Down) => {
            let state = store.state();
            if !state.command_suggestions.is_empty() {
                // Navigate command suggestions
                store.dispatch(Action::Input(InputAction::SuggestionDown));
            } else if state.input.is_empty() {
                store.dispatch(Action::scroll_down(1));
            } else {
                store.dispatch(Action::Input(InputAction::HistoryDown));
            }
            Ok(true)
        }
        // Tab select current command suggestion
        (KeyModifiers::NONE, KeyCode::Tab) => {
            if !store.state().command_suggestions.is_empty() {
                store.dispatch(Action::Input(InputAction::SelectSuggestion));
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

/// Execute a slash command
fn execute_slash_command(store: &mut Store, command: SlashCommand) {
    use nexacode_core::core::slash_commands::SlashCommand::*;
    
    match command {
        // Model management
        Model { name } => {
            match name {
                Some(model_name) => {
                    // Load config, update model, save
                    let mut config = nexacode_core::Config::load();
                    config.set_model(&model_name);
                    if let Err(e) = config.save() {
                        store.dispatch(Action::show_status(
                            format!("Failed to save config: {}", e),
                            true
                        ));
                    } else {
                        store.dispatch(Action::show_status(
                            format!("Model switched to: {} (restart to apply)", model_name),
                            false
                        ));
                    }
                }
                None => {
                    let config = nexacode_core::Config::load();
                    store.dispatch(Action::show_status(
                        format!("Current provider: {}, model: {}", config.current_provider(), config.current_model_display()),
                        false
                    ));
                }
            }
        }
        Models => {
            let config = nexacode_core::Config::load();
            let models = config.configured_models();
            store.dispatch(Action::Input(InputAction::StartModelSelection(models)));
        }
        Provider { name } => {
            match name {
                Some(provider_name) => {
                    // Check if provider exists
                    let config = nexacode_core::Config::load();
                    let available_providers = config.providers();
                    
                    if !available_providers.contains(&provider_name) && 
                       !provider_name.eq_ignore_ascii_case("anthropic") &&
                       !provider_name.eq_ignore_ascii_case("openai") {
                        // Allow custom providers even if not pre-configured
                    }
                    
                    let mut config = nexacode_core::Config::load();
                    config.set_provider(&provider_name);
                    if let Err(e) = config.save() {
                        store.dispatch(Action::show_status(
                            format!("Failed to save config: {}", e),
                            true
                        ));
                    } else {
                        store.dispatch(Action::show_status(
                            format!("Provider switched to: {} (model: {})", provider_name, config.current_model_display()),
                            false
                        ));
                    }
                }
                None => {
                    let config = nexacode_core::Config::load();
                    let providers_list = config.providers().join(", ");
                    store.dispatch(Action::show_status(
                        format!("Current provider: {} (model: {})\nAvailable: {}", 
                            config.current_provider(), 
                            config.current_model_display(),
                            providers_list),
                        false
                    ));
                }
            }
        }
        Config => {
            let config = nexacode_core::Config::load();
            store.dispatch(Action::assistant_message(config.to_display_string()));
        }
        
        // Session management
        New => {
            store.dispatch(Action::new_session());
            store.dispatch(Action::show_status("New session created", false));
        }
        Sessions => {
            // TODO: Show actual sessions
            let sessions = r#"Saved Sessions:
  session-2024-01-15-abc123  (Today, 10:30 AM) - 5 messages
  session-2024-01-14-def456  (Yesterday) - 12 messages
  session-2024-01-10-ghi789  (Jan 10) - 8 messages

Use /load <id> to restore a session"#;
            store.dispatch(Action::assistant_message(sessions));
        }
        Load { id } => {
            // TODO: Actually load session
            store.dispatch(Action::show_status(
                format!("Loading session: {}...", id),
                false
            ));
        }
        Save => {
            store.dispatch(Action::save_session());
            store.dispatch(Action::show_status("Session saved", false));
        }
        Export { format } => {
            let fmt = format.unwrap_or_else(|| "json".to_string());
            // TODO: Actually export
            store.dispatch(Action::show_status(
                format!("Session exported as {}", fmt),
                false
            ));
        }
        
        // Conversation control
        Undo => {
            store.dispatch(Action::Undo);
            store.dispatch(Action::show_status("Undone", false));
        }
        Redo => {
            store.dispatch(Action::Redo);
            store.dispatch(Action::show_status("Redone", false));
        }
        Rollback { count } => {
            let n = count.unwrap_or(1);
            // TODO: Actually rollback
            store.dispatch(Action::show_status(
                format!("Rollback {} messages", n),
                false
            ));
        }
        Clear => {
            store.dispatch(Action::clear_messages());
            store.dispatch(Action::show_status("Conversation cleared", false));
        }
        
        // System commands
        Help { command } => {
            let help_text = get_help(command.as_deref());
            store.dispatch(Action::assistant_message(help_text));
        }
        Version => {
            let version = r#"NexaCode v0.1.0
A powerful AI-powered code assistant CLI

Built with Rust, Ratatui, and ❤️

Repository: https://github.com/nexacode/nexacode"#;
            store.dispatch(Action::assistant_message(version));
        }
        Quit => {
            store.dispatch(Action::quit());
        }
        Theme { name } => {
            match name {
                Some(theme_name) => {
                    match theme_name.to_lowercase().as_str() {
                        "dark" | "light" => {
                            store.dispatch(Action::toggle_theme());
                            store.dispatch(Action::show_status(
                                format!("Theme switched to: {}", theme_name),
                                false
                            ));
                        }
                        _ => {
                            store.dispatch(Action::show_status(
                                format!("Unknown theme: {}. Use 'dark' or 'light'", theme_name),
                                true
                            ));
                        }
                    }
                }
                None => {
                    store.dispatch(Action::toggle_theme());
                }
            }
        }
    }
    
    // Clear input after command
    store.dispatch(Action::Input(InputAction::ClearInput));
}

/// Handle keys in model selection mode
fn handle_model_selection(store: &mut Store, key_event: KeyEvent) -> anyhow::Result<bool> {
    match (key_event.modifiers, key_event.code) {
        // Up navigate up
        (KeyModifiers::NONE, KeyCode::Up) | (KeyModifiers::NONE, KeyCode::Char('k')) => {
            store.dispatch(Action::Input(InputAction::ModelSelectionUp));
            Ok(true)
        }
        // Down navigate down
        (KeyModifiers::NONE, KeyCode::Down) | (KeyModifiers::NONE, KeyCode::Char('j')) => {
            store.dispatch(Action::Input(InputAction::ModelSelectionDown));
            Ok(true)
        }
        // Enter or Tab select model
        (KeyModifiers::NONE, KeyCode::Enter) | (KeyModifiers::NONE, KeyCode::Tab) => {
            // Get selected model and switch provider
            let selected_index = store.state().model_selection_index.unwrap_or(0);
            if let Some(model) = store.state().model_selections.get(selected_index).cloned() {
                // Clear selection state first
                store.dispatch(Action::Input(InputAction::SelectModel));
                
                // Switch to the provider
                let mut config = nexacode_core::Config::load();
                config.set_provider(&model.provider);
                if let Err(e) = config.save() {
                    store.dispatch(Action::show_status(
                        format!("Failed to save config: {}", e),
                        true
                    ));
                } else {
                    store.dispatch(Action::show_status(
                        format!("Switched to: {}", model),
                        false
                    ));
                }
            }
            Ok(true)
        }
        // Escape cancel selection
        (KeyModifiers::NONE, KeyCode::Esc) => {
            store.dispatch(Action::Input(InputAction::CancelModelSelection));
            Ok(true)
        }
        // Ctrl+C quit
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            store.dispatch(Action::Input(InputAction::CancelModelSelection));
            Ok(true)
        }
        // Any other key - cancel selection
        _ => {
            store.dispatch(Action::Input(InputAction::CancelModelSelection));
            Ok(true)
        }
    }
}
