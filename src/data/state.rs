//! Pure data types for application state.

use std::collections::HashSet;
use std::path::PathBuf;

use ratatui::style::Color;

use super::definition::Testlist;
use super::results::TestlistResults;

/// Which pane is currently focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusedPane {
    #[default]
    Tests,
    Notes,
    Terminal,
}

impl FocusedPane {
    pub fn next(self) -> Self {
        match self {
            FocusedPane::Tests => FocusedPane::Notes,
            FocusedPane::Notes => FocusedPane::Terminal,
            FocusedPane::Terminal => FocusedPane::Tests,
        }
    }
}

/// What is selected within an expanded test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubSelection {
    /// The test header row itself
    Header,
    /// A setup checklist item (index)
    Setup(usize),
    /// The action line
    Action,
    /// A verify checklist item (index)
    Verify(usize),
}

/// Theme for the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl Theme {
    pub fn toggle(self) -> Self {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        }
    }

    pub fn bg(self) -> Color {
        match self {
            Theme::Dark => Color::Black,
            Theme::Light => Color::White,
        }
    }

    pub fn fg(self) -> Color {
        match self {
            Theme::Dark => Color::White,
            Theme::Light => Color::Black,
        }
    }

    pub fn dim(self) -> Color {
        match self {
            Theme::Dark => Color::DarkGray,
            Theme::Light => Color::Gray,
        }
    }

    pub fn selection_bg(self) -> Color {
        match self {
            Theme::Dark => Color::DarkGray,
            Theme::Light => Color::LightBlue,
        }
    }

    pub fn accent(self) -> Color {
        match self {
            Theme::Dark => Color::Cyan,
            Theme::Light => Color::Blue,
        }
    }
}

/// Pure application state — no methods with side effects.
pub struct AppState {
    pub testlist: Testlist,
    pub results: TestlistResults,
    pub testlist_path: PathBuf,
    pub results_path: PathBuf,
    pub selected_test: usize,
    pub sub_selection: SubSelection,
    pub focused_pane: FocusedPane,
    pub expanded_tests: HashSet<String>,
    pub should_quit: bool,
    // Notes editing state
    pub editing_notes: bool,
    pub notes_input: String,
    pub adding_screenshot: bool,
    pub screenshot_input: String,
    // Terminal size tracking
    pub terminal_size: (u16, u16),
    // Scroll offset for tests pane
    pub tests_scroll_offset: usize,
    // Visible height of tests pane (updated during draw)
    pub tests_visible_height: usize,
    // Track unsaved changes
    pub dirty: bool,
    // Show quit confirmation dialog
    pub confirm_quit: bool,
    // UI theme
    pub theme: Theme,
}

impl AppState {
    pub fn new(
        testlist: Testlist,
        results: TestlistResults,
        testlist_path: PathBuf,
        results_path: PathBuf,
    ) -> Self {
        Self {
            testlist,
            results,
            testlist_path,
            results_path,
            selected_test: 0,
            sub_selection: SubSelection::Header,
            focused_pane: FocusedPane::Tests,
            expanded_tests: HashSet::new(),
            should_quit: false,
            editing_notes: false,
            notes_input: String::new(),
            adding_screenshot: false,
            screenshot_input: String::new(),
            terminal_size: (24, 80),
            tests_scroll_offset: 0,
            tests_visible_height: 20,
            dirty: false,
            confirm_quit: false,
            theme: Theme::Dark,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_default_is_dark() {
        assert_eq!(Theme::default(), Theme::Dark);
    }

    #[test]
    fn test_theme_toggle() {
        assert_eq!(Theme::Dark.toggle(), Theme::Light);
        assert_eq!(Theme::Light.toggle(), Theme::Dark);
    }

    #[test]
    fn test_theme_colors_differ() {
        assert_ne!(Theme::Dark.bg(), Theme::Light.bg());
        assert_ne!(Theme::Dark.fg(), Theme::Light.fg());
        assert_ne!(Theme::Dark.selection_bg(), Theme::Light.selection_bg());
    }

    #[test]
    fn test_focused_pane_next() {
        assert_eq!(FocusedPane::Tests.next(), FocusedPane::Notes);
        assert_eq!(FocusedPane::Notes.next(), FocusedPane::Terminal);
        assert_eq!(FocusedPane::Terminal.next(), FocusedPane::Tests);
    }

    #[test]
    fn test_focused_pane_default() {
        assert_eq!(FocusedPane::default(), FocusedPane::Tests);
    }

    #[test]
    fn test_sub_selection_equality() {
        assert_eq!(SubSelection::Header, SubSelection::Header);
        assert_eq!(SubSelection::Setup(0), SubSelection::Setup(0));
        assert_ne!(SubSelection::Setup(0), SubSelection::Setup(1));
        assert_eq!(SubSelection::Action, SubSelection::Action);
        assert_eq!(SubSelection::Verify(2), SubSelection::Verify(2));
        assert_ne!(SubSelection::Setup(0), SubSelection::Verify(0));
    }
}
