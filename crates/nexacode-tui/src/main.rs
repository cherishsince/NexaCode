//! NexaCode - A cross-platform terminal Code Agent
//!
//! See ARCHITECTURE.md for detailed design.

use anyhow::Result;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use nexacode_tui::{handle_event, render, Store};
use nexacode_core::core::agent::AgentController;
use nexacode_core::Config;
use ratatui::prelude::*;
use std::io::{self, Stdout};
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "nexacode=info".into()),
        )
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

    // Run the app
    let result = run_app(&mut terminal, &mut store, agent).await;

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

async fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>, store: &mut Store, agent: Arc<AgentController>) -> Result<()> {
    while !store.state().should_quit {
        terminal.draw(|f| render(f, f.size(), store.state()))?;
        handle_event(store, agent.clone()).await?;
    }

    Ok(())
}
