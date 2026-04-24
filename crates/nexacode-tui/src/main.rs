//! NexaCode - A cross-platform terminal Code Agent
//!
//! See ARCHITECTURE.md for detailed design.

use anyhow::Result;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use nexacode_tui::{handle_event, render, Store};
use ratatui::prelude::*;
use std::io::{self, Stdout};
use tracing::info;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "nexacode=info".into()),
        )
        .init();

    info!("Starting NexaCode v{}", env!("CARGO_PKG_VERSION"));

    // Setup terminal
    let mut terminal = setup_terminal()?;

    // Create state store
    let mut store = Store::new();

    // Run the app
    let result = run_app(&mut terminal, &mut store);

    // Restore terminal
    restore_terminal(&mut terminal)?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
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

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>, store: &mut Store) -> Result<()> {
    while !store.state().should_quit {
        terminal.draw(|f| render(f, f.size(), store.state()))?;
        pollster::block_on(handle_event(store))?;
    }

    Ok(())
}
