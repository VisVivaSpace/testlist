//! Side-effect descriptions returned by transforms.

/// Effects that the UI layer should execute.
/// Transforms return these instead of performing side effects directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Save the current results to disk.
    SaveResults,
    /// Quit the application.
    Quit,
    /// Insert a command string into the embedded terminal.
    InsertTerminalCommand(String),
}
