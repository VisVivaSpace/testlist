//! Application setup, teardown, and main entry point.

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::Terminal;
use std::io::stdout;

use crate::data::state::AppState;
use crate::error::Result;
use crate::ui::panes::terminal::EmbeddedTerminal;

/// Run the TUI application.
pub fn run(state: &mut AppState) -> Result<()> {
    // Create embedded terminal (may fail on some systems)
    let mut terminal_pty = EmbeddedTerminal::new(24, 80).ok();

    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    stdout().execute(EnableMouseCapture)?;
    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout()))?;

    // Main loop
    let result = super::main_loop(&mut terminal, state, &mut terminal_pty);

    // Restore terminal
    stdout().execute(DisableMouseCapture)?;
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    result
}
