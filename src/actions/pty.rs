//! PTY terminal creation and management.
//!
//! The EmbeddedTerminal struct lives here conceptually but is defined
//! in the UI layer (src/ui/panes/terminal.rs) since it manages the PTY
//! lifecycle directly. This module provides helper functions for PTY operations
//! that can be called from transforms or actions.

/// Send a command string to the terminal (called from UI layer).
/// This is a thin wrapper documenting the intent — actual sending
/// happens through EmbeddedTerminal::send_str in the UI layer.
pub fn prepare_command(suggested_command: Option<&str>) -> Option<String> {
    suggested_command.map(|s| s.to_string())
}
