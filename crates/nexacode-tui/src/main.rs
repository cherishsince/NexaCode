//! NexaCode - A cross-platform terminal Code Agent
//!
//! See ARCHITECTURE.md for detailed design.

use anyhow::Result;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use crossterm::event::{Event, KeyEvent, KeyEventKind};
use nexacode_tui::{render, Store};
use nexacode_core::core::agent::AgentController;
use nexacode_core::core::slash_commands::SlashCommand;
use nexacode_core::Config;
use nexacode_core::NexaCodeDir;
use ratatui::prelude::*;
use std::io::{self, Stdout};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Message type for streaming responses
#[derive(Debug, Clone)]
pub enum StreamMessage {
    /// A chunk of the response
    Chunk(String),
    /// Response is complete
    Complete,
    /// An error occurred
    Error(String),
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to file
    let data_dir = NexaCodeDir::new();
    let log_dir = data_dir.logs_dir();
    
    let file_appender = tracing_appender::rolling::daily(log_dir, "nexacode.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "nexacode=debug".into())
        )
        .with(tracing_subscriber::fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false))
        .init();

    info!("Starting NexaCode v{}", env!("CARGO_PKG_VERSION"));

    // Initialize data directory if this is a first run
    initialize_data_directory()?;

    // Load config and create agent
    let config = Config::load();
    let agent = Arc::new(AgentController::new(config.llm.clone()));

    // Setup terminal
    let mut terminal = setup_terminal()?;

    // Create state store
    let mut store = Store::new();

    // Create channel for streaming messages
    let (stream_tx, mut stream_rx) = mpsc::unbounded_channel::<StreamMessage>();

    // Run the app
    let result = run_app(&mut terminal, &mut store, agent, &mut stream_rx, stream_tx.clone()).await;

    // Restore terminal
    restore_terminal(&mut terminal)?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

/// Initialize the NexaCode data directory on first run
fn initialize_data_directory() -> Result<()> {
    use nexacode_core::NexaCodeDir;

    let data_dir = NexaCodeDir::new();

    if data_dir.is_first_run() {
        info!("First run detected - initializing data directory");
        data_dir.initialize()?;
        info!("Data directory created at: {:?}", data_dir.root());
    } else {
        // Ensure directories exist even if not first run
        data_dir.ensure_dirs()?;
    }

    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    store: &mut Store,
    agent: Arc<AgentController>,
    stream_rx: &mut mpsc::UnboundedReceiver<StreamMessage>,
    stream_tx: mpsc::UnboundedSender<StreamMessage>,
) -> Result<()> {
    use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
    use std::time::Duration;
    
    while !store.state().should_quit {
        // Check for stream messages first (non-blocking) - process one at a time for smoother display
        if let Ok(msg) = stream_rx.try_recv() {
            use nexacode_core::{Action, MessageAction};
            match msg {
                StreamMessage::Chunk(chunk) => {
                    tracing::debug!("Received stream chunk: {} chars", chunk.len());
                    store.dispatch(Action::Message(MessageAction::AppendToLastMessage(chunk)));
                }
                StreamMessage::Complete => {
                    tracing::info!("Stream complete");
                }
                StreamMessage::Error(err) => {
                    tracing::error!("Stream error: {}", err);
                    store.dispatch(Action::show_status(err, true));
                }
            }
            
            // Immediately draw after receiving a chunk for real-time display
            terminal.draw(|f| render(f, f.size(), store.state()))?;
            
            // Drain any remaining chunks to avoid backlog, but draw after each
            while let Ok(msg) = stream_rx.try_recv() {
                match msg {
                    StreamMessage::Chunk(chunk) => {
                        store.dispatch(Action::Message(MessageAction::AppendToLastMessage(chunk)));
                        terminal.draw(|f| render(f, f.size(), store.state()))?;
                    }
                    StreamMessage::Complete => {
                        tracing::info!("Stream complete");
                    }
                    StreamMessage::Error(err) => {
                        tracing::error!("Stream error: {}", err);
                        store.dispatch(Action::show_status(err, true));
                    }
                }
            }
        }
        
        // Draw UI
        terminal.draw(|f| render(f, f.size(), store.state()))?;
        
        // Poll for terminal events with a short timeout
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    handle_key_event(store, key_event, agent.clone(), stream_tx.clone()).await?;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

async fn handle_key_event(
    store: &mut Store,
    key_event: KeyEvent,
    agent: Arc<AgentController>,
    stream_tx: mpsc::UnboundedSender<StreamMessage>,
) -> Result<()> {
    use crossterm::event::{KeyCode, KeyModifiers};
    use nexacode_core::{Action, InputAction, MessageAction, Mode};
    use nexacode_core::core::slash_commands::{parse_slash_command, SlashCommand};

    // Handle model selection mode
    if !store.state().model_selections.is_empty() {
        return handle_model_selection(store, key_event);
    }

    let state = store.state();

    // Handle keys based on current mode
    match state.mode {
        Mode::Normal => handle_normal_mode(store, key_event),
        Mode::Input => handle_input_mode(store, key_event, agent, stream_tx).await,
        Mode::Command => handle_command_mode(store, key_event),
        Mode::Search => handle_search_mode(store, key_event),
    }
}

fn handle_model_selection(store: &mut Store, key_event: KeyEvent) -> Result<()> {
    use crossterm::event::{KeyCode, KeyModifiers};
    use nexacode_core::{Action, InputAction};
    use nexacode_core::Config;

    match (key_event.modifiers, key_event.code) {
        // Up navigate up
        (KeyModifiers::NONE, KeyCode::Up) | (KeyModifiers::NONE, KeyCode::Char('k')) => {
            store.dispatch(Action::Input(InputAction::ModelSelectionUp));
        }
        // Down navigate down
        (KeyModifiers::NONE, KeyCode::Down) | (KeyModifiers::NONE, KeyCode::Char('j')) => {
            store.dispatch(Action::Input(InputAction::ModelSelectionDown));
        }
        // Enter or Tab select model
        (KeyModifiers::NONE, KeyCode::Enter) | (KeyModifiers::NONE, KeyCode::Tab) => {
            let selected_index = store.state().model_selection_index.unwrap_or(0);
            if let Some(model) = store.state().model_selections.get(selected_index).cloned() {
                store.dispatch(Action::Input(InputAction::SelectModel));
                
                let mut config = Config::load();
                config.set_provider(&model.provider);
                if let Err(e) = config.save() {
                    store.dispatch(Action::show_status(format!("Failed to save config: {}", e), true));
                } else {
                    store.dispatch(Action::show_status(format!("Switched to: {}", model), false));
                }
            }
        }
        // Escape cancel selection
        (KeyModifiers::NONE, KeyCode::Esc) => {
            store.dispatch(Action::Input(InputAction::CancelModelSelection));
        }
        _ => {
            store.dispatch(Action::Input(InputAction::CancelModelSelection));
        }
    }
    Ok(())
}

fn handle_normal_mode(store: &mut Store, key_event: KeyEvent) -> Result<()> {
    use crossterm::event::{KeyCode, KeyModifiers};
    use nexacode_core::{Action, Mode};

    match (key_event.modifiers, key_event.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            store.dispatch(Action::quit());
        }
        (KeyModifiers::NONE, KeyCode::Char('q')) => {
            store.dispatch(Action::quit());
        }
        (KeyModifiers::NONE, KeyCode::Char('i')) | (KeyModifiers::NONE, KeyCode::Char('a')) => {
            store.dispatch(Action::set_mode(Mode::Input));
        }
        (KeyModifiers::NONE, KeyCode::Char(':')) => {
            store.dispatch(Action::set_mode(Mode::Command));
        }
        (KeyModifiers::NONE, KeyCode::Char('/')) => {
            store.dispatch(Action::set_mode(Mode::Search));
        }
        (KeyModifiers::NONE, KeyCode::Char('t')) | (KeyModifiers::CONTROL, KeyCode::Char('t')) => {
            store.dispatch(Action::toggle_theme());
        }
        (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
            store.dispatch(Action::scroll_down(1));
        }
        (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
            store.dispatch(Action::scroll_up(1));
        }
        (KeyModifiers::NONE, KeyCode::Char('g')) => {
            store.dispatch(Action::scroll_to_top());
        }
        (KeyModifiers::NONE, KeyCode::Char('G')) => {
            store.dispatch(Action::scroll_to_bottom());
        }
        (KeyModifiers::NONE, KeyCode::Char('h')) => {
            store.dispatch(Action::toggle_help());
        }
        (KeyModifiers::SHIFT, KeyCode::Char('?')) => {
            store.dispatch(Action::toggle_help());
        }
        (KeyModifiers::NONE, KeyCode::Char(' ')) | (KeyModifiers::NONE, KeyCode::Enter) => {
            store.dispatch(Action::set_mode(Mode::Input));
        }
        _ => {}
    }
    Ok(())
}

async fn handle_input_mode(
    store: &mut Store,
    key_event: KeyEvent,
    agent: Arc<AgentController>,
    stream_tx: mpsc::UnboundedSender<StreamMessage>,
) -> Result<()> {
    use crossterm::event::{KeyCode, KeyModifiers};
    use nexacode_core::{Action, InputAction, MessageAction, Mode};
    use nexacode_core::core::slash_commands::{parse_slash_command, SlashCommand};
    use nexacode_core::Config;

    match (key_event.modifiers, key_event.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            store.dispatch(Action::quit());
        }
        (KeyModifiers::NONE, KeyCode::Esc) => {
            store.dispatch(Action::set_mode(Mode::Normal));
        }
        (KeyModifiers::NONE, KeyCode::Enter) => {
            if !store.state().command_suggestions.is_empty() {
                let selected_index = store.state().command_suggestion_index.unwrap_or(0);
                if let Some(suggestion) = store.state().command_suggestions.get(selected_index).cloned() {
                    store.dispatch(Action::Input(InputAction::ClearInput));
                    match parse_slash_command(&suggestion) {
                        nexacode_core::core::slash_commands::ParseResult::Ok(cmd) => {
                            execute_slash_command(store, cmd);
                        }
                        _ => {
                            store.dispatch(Action::Input(InputAction::SetInput(suggestion)));
                        }
                    }
                }
            } else {
                let content = store.state().input.clone();
                if !content.trim().is_empty() {
                    match parse_slash_command(&content) {
                        nexacode_core::core::slash_commands::ParseResult::Ok(cmd) => {
                            execute_slash_command(store, cmd);
                        }
                        nexacode_core::core::slash_commands::ParseResult::NotACommand(msg) => {
                            // Add user message
                            store.dispatch(Action::user_message(msg.clone()));
                            
                            // Add empty assistant message for streaming
                            store.dispatch(Action::assistant_message(""));
                            
                            // Spawn a task to handle streaming
                            let agent_clone = agent.clone();
                            let tx_clone = stream_tx.clone();
                            let tx_for_complete = stream_tx.clone();
                            tokio::spawn(async move {
                                tracing::info!("Starting streaming task for message: {}", msg);
                                
                                // Create callback that sends chunks via channel
                                let callback = Box::new(move |chunk: &str| {
                                    tracing::debug!("Callback sending chunk: {} chars", chunk.len());
                                    let _ = tx_clone.send(StreamMessage::Chunk(chunk.to_string()));
                                });
                                
                                match agent_clone.process_user_message_stream(msg, callback).await {
                                    Ok(_) => {
                                        tracing::info!("Streaming task completed successfully");
                                        let _ = tx_for_complete.send(StreamMessage::Complete);
                                    }
                                    Err(e) => {
                                        tracing::error!("Streaming task failed: {}", e);
                                        let _ = tx_for_complete.send(StreamMessage::Error(e.to_string()));
                                    }
                                }
                            });
                        }
                        nexacode_core::core::slash_commands::ParseResult::Error(err) => {
                            store.dispatch(Action::show_status(err, true));
                        }
                    }
                }
            }
            store.dispatch(Action::Input(InputAction::ClearInput));
        }
        (KeyModifiers::CONTROL, KeyCode::Char('t')) => {
            store.dispatch(Action::toggle_theme());
        }
        (KeyModifiers::NONE, KeyCode::Char(c)) => {
            store.dispatch(Action::insert_char(c));
        }
        (KeyModifiers::NONE, KeyCode::Backspace) => {
            store.dispatch(Action::delete_char());
        }
        (KeyModifiers::NONE, KeyCode::Up) => {
            if !store.state().command_suggestions.is_empty() {
                store.dispatch(Action::Input(InputAction::SuggestionUp));
            }
        }
        (KeyModifiers::NONE, KeyCode::Down) => {
            if !store.state().command_suggestions.is_empty() {
                store.dispatch(Action::Input(InputAction::SuggestionDown));
            }
        }
        (KeyModifiers::NONE, KeyCode::Tab) => {
            if !store.state().command_suggestions.is_empty() {
                store.dispatch(Action::Input(InputAction::SelectSuggestion));
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_command_mode(store: &mut Store, key_event: KeyEvent) -> Result<()> {
    use crossterm::event::{KeyCode, KeyModifiers};
    use nexacode_core::{Action, InputAction, Mode};

    match (key_event.modifiers, key_event.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            store.dispatch(Action::quit());
        }
        (KeyModifiers::NONE, KeyCode::Esc) => {
            store.dispatch(Action::set_mode(Mode::Normal));
        }
        (KeyModifiers::NONE, KeyCode::Enter) => {
            let content = store.state().input.clone();
            store.dispatch(Action::Input(InputAction::ClearInput));
            store.dispatch(Action::set_mode(Mode::Normal));
            // Handle command
            handle_colon_command(store, &content);
        }
        (KeyModifiers::NONE, KeyCode::Char(c)) => {
            store.dispatch(Action::insert_char(c));
        }
        (KeyModifiers::NONE, KeyCode::Backspace) => {
            store.dispatch(Action::delete_char());
        }
        _ => {}
    }
    Ok(())
}

fn handle_search_mode(store: &mut Store, key_event: KeyEvent) -> Result<()> {
    use crossterm::event::{KeyCode, KeyModifiers};
    use nexacode_core::{Action, InputAction, Mode};

    match (key_event.modifiers, key_event.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            store.dispatch(Action::quit());
        }
        (KeyModifiers::NONE, KeyCode::Esc) => {
            store.dispatch(Action::clear_search());
            store.dispatch(Action::set_mode(Mode::Normal));
        }
        (KeyModifiers::NONE, KeyCode::Enter) => {
            let query = store.state().input.clone();
            if !query.is_empty() {
                store.dispatch(Action::search(query));
            }
            store.dispatch(Action::Input(InputAction::ClearInput));
            store.dispatch(Action::set_mode(Mode::Normal));
        }
        (KeyModifiers::NONE, KeyCode::Char(c)) => {
            store.dispatch(Action::insert_char(c));
        }
        (KeyModifiers::NONE, KeyCode::Backspace) => {
            store.dispatch(Action::delete_char());
        }
        _ => {}
    }
    Ok(())
}

fn handle_colon_command(store: &mut Store, cmd: &str) {
    use nexacode_core::{Action};
    let cmd = cmd.trim_start_matches(':').trim();
    
    match cmd {
        "q" | "quit" | "exit" => {
            store.dispatch(Action::quit());
        }
        "h" | "help" => {
            store.dispatch(Action::toggle_help());
        }
        "clear" => {
            store.dispatch(Action::clear_messages());
        }
        "theme" => {
            store.dispatch(Action::toggle_theme());
        }
        _ => {
            store.dispatch(Action::show_status(format!("Unknown command: {}", cmd), true));
        }
    }
}

fn execute_slash_command(store: &mut Store, command: SlashCommand) {
    use nexacode_core::{Action, InputAction, Config};
    use nexacode_core::core::slash_commands::SlashCommand::*;

    match command {
        Model { name } => {
            match name {
                Some(model_name) => {
                    let mut config = Config::load();
                    config.set_model(&model_name);
                    if let Err(e) = config.save() {
                        store.dispatch(Action::show_status(format!("Failed to save config: {}", e), true));
                    } else {
                        store.dispatch(Action::show_status(format!("Model switched to: {}", model_name), false));
                    }
                }
                None => {
                    let config = Config::load();
                    store.dispatch(Action::show_status(
                        format!("Current provider: {}, model: {}", config.current_provider(), config.current_model_display()),
                        false
                    ));
                }
            }
        }
        Models => {
            let config = Config::load();
            let models = config.configured_models();
            store.dispatch(Action::Input(InputAction::StartModelSelection(models)));
        }
        Provider { name } => {
            match name {
                Some(provider_name) => {
                    let mut config = Config::load();
                    config.set_provider(&provider_name);
                    if let Err(e) = config.save() {
                        store.dispatch(Action::show_status(format!("Failed to save config: {}", e), true));
                    } else {
                        store.dispatch(Action::show_status(
                            format!("Provider switched to: {} (model: {})", provider_name, config.current_model_display()),
                            false
                        ));
                    }
                }
                None => {
                    let config = Config::load();
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
            let config = Config::load();
            store.dispatch(Action::assistant_message(config.to_display_string()));
        }
        Help { command } => {
            let help_text = nexacode_core::core::slash_commands::get_help(command.as_deref());
            store.dispatch(Action::assistant_message(help_text));
        }
        Clear => {
            store.dispatch(Action::clear_messages());
            store.dispatch(Action::show_status("Conversation cleared", false));
        }
        Quit => {
            store.dispatch(Action::quit());
        }
        _ => {
            store.dispatch(Action::show_status("Command not implemented", true));
        }
    }
    
    store.dispatch(Action::Input(InputAction::ClearInput));
}
