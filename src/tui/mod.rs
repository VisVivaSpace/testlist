//! TUI application for testlist.

mod terminal;

use crate::error::Result;
use crate::schema::{Results, Status, Testlist};
use terminal::EmbeddedTerminal;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::io::stdout;
use std::path::PathBuf;

/// Which pane is currently focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusedPane {
    #[default]
    Tests,
    Notes,
    Terminal,
}

impl FocusedPane {
    fn next(self) -> Self {
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
    fn toggle(self) -> Self {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        }
    }

    /// Get the background color for the theme.
    fn bg(self) -> Color {
        match self {
            Theme::Dark => Color::Black,
            Theme::Light => Color::White,
        }
    }

    /// Get the foreground color for the theme.
    fn fg(self) -> Color {
        match self {
            Theme::Dark => Color::White,
            Theme::Light => Color::Black,
        }
    }

    /// Get the dim/muted color for the theme.
    fn dim(self) -> Color {
        match self {
            Theme::Dark => Color::DarkGray,
            Theme::Light => Color::Gray,
        }
    }

    /// Get the selection/highlight background color.
    fn selection_bg(self) -> Color {
        match self {
            Theme::Dark => Color::DarkGray,
            Theme::Light => Color::LightBlue,
        }
    }

    /// Get the accent color (for focused borders, etc).
    fn accent(self) -> Color {
        match self {
            Theme::Dark => Color::Cyan,
            Theme::Light => Color::Blue,
        }
    }
}

/// Application state for the TUI.
pub struct AppState {
    pub testlist: Testlist,
    pub results: Results,
    pub testlist_path: PathBuf,
    pub results_path: PathBuf,
    pub selected_test: usize,
    pub sub_selection: SubSelection,
    pub focused_pane: FocusedPane,
    pub expanded_tests: std::collections::HashSet<String>,
    pub should_quit: bool,
    // Notes editing state
    pub editing_notes: bool,
    pub notes_input: String,
    pub adding_screenshot: bool,
    pub screenshot_input: String,
    // Embedded terminal
    pub terminal: Option<EmbeddedTerminal>,
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
    /// Create a new AppState from a loaded testlist.
    pub fn new(
        testlist: Testlist,
        results: Results,
        testlist_path: PathBuf,
        results_path: PathBuf,
    ) -> Self {
        // Try to create embedded terminal (may fail on some systems)
        let terminal = EmbeddedTerminal::new(24, 80).ok();

        Self {
            testlist,
            results,
            testlist_path,
            results_path,
            selected_test: 0,
            sub_selection: SubSelection::Header,
            focused_pane: FocusedPane::Tests,
            expanded_tests: std::collections::HashSet::new(),
            should_quit: false,
            editing_notes: false,
            notes_input: String::new(),
            adding_screenshot: false,
            screenshot_input: String::new(),
            terminal,
            terminal_size: (24, 80),
            tests_scroll_offset: 0,
            tests_visible_height: 20,
            dirty: false,
            confirm_quit: false,
            theme: Theme::Dark,
        }
    }

    /// Get the currently selected test.
    pub fn current_test(&self) -> Option<&crate::schema::Test> {
        self.testlist.tests.get(self.selected_test)
    }

    /// Get the result for the currently selected test.
    pub fn current_result(&self) -> Option<&crate::schema::TestResult> {
        self.current_test()
            .and_then(|t| self.results.results.iter().find(|r| r.test_id == t.id))
    }

    /// Get mutable result for the currently selected test.
    pub fn current_result_mut(&mut self) -> Option<&mut crate::schema::TestResult> {
        let test_id = self.testlist.tests.get(self.selected_test)?.id.clone();
        self.results.get_result_mut(&test_id)
    }

    /// Count completed tests.
    pub fn completed_count(&self) -> usize {
        self.results
            .results
            .iter()
            .filter(|r| r.status != Status::Pending)
            .count()
    }

    /// Calculate the line number of the current selection in the tests pane.
    fn selected_line_number(&self) -> usize {
        let mut line = 0;

        for (i, test) in self.testlist.tests.iter().enumerate() {
            if i == self.selected_test && self.sub_selection == SubSelection::Header {
                return line;
            }
            line += 1;

            if self.expanded_tests.contains(&test.id) {
                // Setup section
                if !test.setup.is_empty() {
                    line += 1; // "Setup:" header
                    for j in 0..test.setup.len() {
                        if i == self.selected_test && self.sub_selection == SubSelection::Setup(j) {
                            return line;
                        }
                        line += 1;
                    }
                }

                // Action
                if i == self.selected_test && self.sub_selection == SubSelection::Action {
                    return line;
                }
                line += 1;

                // Verify section
                if !test.verify.is_empty() {
                    line += 1; // "Verify:" header
                    for j in 0..test.verify.len() {
                        if i == self.selected_test && self.sub_selection == SubSelection::Verify(j) {
                            return line;
                        }
                        line += 1;
                    }
                }
            }
        }

        line
    }

    /// Adjust scroll offset to keep selection visible.
    pub fn adjust_scroll(&mut self) {
        let selected = self.selected_line_number();
        let visible = self.tests_visible_height;

        // Scroll up if selection is above visible area
        if selected < self.tests_scroll_offset {
            self.tests_scroll_offset = selected;
        }
        // Scroll down if selection is below visible area
        else if selected >= self.tests_scroll_offset + visible {
            self.tests_scroll_offset = selected.saturating_sub(visible) + 1;
        }
    }
}

/// Run the TUI application.
pub fn run(state: &mut AppState) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    stdout().execute(EnableMouseCapture)?;
    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout()))?;

    // Main loop
    let result = main_loop(&mut terminal, state);

    // Restore terminal
    stdout().execute(DisableMouseCapture)?;
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    result
}

/// Stores layout information for mouse click handling.
struct LayoutAreas {
    tests_pane: Rect,
    notes_pane: Rect,
    terminal_pane: Rect,
}

fn main_loop(
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    state: &mut AppState,
) -> Result<()> {
    let mut layout_areas: Option<LayoutAreas> = None;

    while !state.should_quit {
        // Poll PTY output
        if let Some(ref mut term) = state.terminal {
            term.poll_output();
        }

        terminal.draw(|frame| {
            layout_areas = Some(draw(frame, state));
        })?;

        // Update visible height from layout areas for scroll calculations
        // Also resize PTY if terminal pane size changed
        if let Some(ref areas) = layout_areas {
            state.tests_visible_height = areas.tests_pane.height.saturating_sub(2) as usize;

            // Resize PTY if terminal pane size changed (accounting for borders)
            let new_rows = areas.terminal_pane.height.saturating_sub(2);
            let new_cols = areas.terminal_pane.width.saturating_sub(2);
            if (new_rows, new_cols) != state.terminal_size {
                state.terminal_size = (new_rows, new_cols);
                if let Some(ref mut term) = state.terminal {
                    term.resize(new_rows, new_cols);
                }
            }
        }

        // Use shorter poll time to keep terminal responsive
        if event::poll(std::time::Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        handle_key(state, key.code, key.modifiers);
                        state.adjust_scroll();
                    }
                }
                Event::Mouse(mouse) => {
                    if let Some(ref areas) = layout_areas {
                        handle_mouse(state, mouse, areas);
                        state.adjust_scroll();
                    }
                }
                Event::Resize(_, _) => {
                    // Terminal resize is handled by ratatui
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn handle_mouse(state: &mut AppState, mouse: crossterm::event::MouseEvent, areas: &LayoutAreas) {
    let x = mouse.column;
    let y = mouse.row;

    // Determine which pane was clicked
    if areas.tests_pane.contains((x, y).into()) {
        state.focused_pane = FocusedPane::Tests;

        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
            // Calculate which test was clicked (accounting for border and scroll)
            let relative_y = y.saturating_sub(areas.tests_pane.y + 1) as usize;
            let absolute_y = relative_y + state.tests_scroll_offset;

            // Map y position to test index, accounting for expanded items
            if let Some(test_idx) = map_y_to_test_index(state, absolute_y) {
                state.selected_test = test_idx;
            }
        }
    } else if areas.notes_pane.contains((x, y).into()) {
        state.focused_pane = FocusedPane::Notes;
    } else if areas.terminal_pane.contains((x, y).into()) {
        state.focused_pane = FocusedPane::Terminal;
    }
}

/// Map a y-coordinate in the tests pane to a test index.
fn map_y_to_test_index(state: &AppState, y: usize) -> Option<usize> {
    let mut current_y = 0;

    for (i, test) in state.testlist.tests.iter().enumerate() {
        if current_y == y {
            return Some(i);
        }
        current_y += 1;

        // Account for expanded content
        if state.expanded_tests.contains(&test.id) {
            // Setup section
            if !test.setup.is_empty() {
                current_y += 2 + test.setup.len(); // header + items + footer
            }
            // Action line
            current_y += 1;
            // Verify section
            if !test.verify.is_empty() {
                current_y += 2 + test.verify.len(); // header + items + footer
            }
        }
    }

    None
}

fn handle_key(state: &mut AppState, key: KeyCode, modifiers: KeyModifiers) {
    // Handle quit confirmation dialog
    if state.confirm_quit {
        match key {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                state.should_quit = true;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                state.confirm_quit = false;
            }
            _ => {}
        }
        return;
    }

    // Handle notes editing mode
    if state.editing_notes {
        handle_notes_editing(state, key);
        return;
    }

    // Handle screenshot path input mode
    if state.adding_screenshot {
        handle_screenshot_input(state, key);
        return;
    }

    // Handle terminal input when focused
    if state.focused_pane == FocusedPane::Terminal && state.terminal.is_some() {
        // Escape exits terminal focus
        if key == KeyCode::Esc {
            state.focused_pane = FocusedPane::Tests;
            return;
        }
        // Tab cycles panes
        if key == KeyCode::Tab {
            state.focused_pane = state.focused_pane.next();
            return;
        }
        handle_terminal_input(state, key, modifiers);
        return;
    }

    // Normal mode key handling
    match key {
        KeyCode::Char('q') => {
            if state.dirty {
                state.confirm_quit = true;
            } else {
                state.should_quit = true;
            }
        }
        KeyCode::Tab => state.focused_pane = state.focused_pane.next(),
        KeyCode::Up | KeyCode::Char('k') => {
            if state.focused_pane == FocusedPane::Tests {
                navigate_up(state);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if state.focused_pane == FocusedPane::Tests {
                navigate_down(state);
            }
        }
        KeyCode::Enter | KeyCode::Char('l') => {
            if state.focused_pane == FocusedPane::Tests {
                // Only toggle expand/collapse when on header
                if state.sub_selection == SubSelection::Header {
                    if let Some(test) = state.current_test() {
                        let id = test.id.clone();
                        if state.expanded_tests.contains(&id) {
                            state.expanded_tests.remove(&id);
                        } else {
                            state.expanded_tests.insert(id);
                        }
                    }
                }
            }
        }
        KeyCode::Char(' ') => {
            // Toggle checklist items with Space
            if state.focused_pane == FocusedPane::Tests {
                toggle_checklist_item(state);
            }
        }
        KeyCode::Char('n') => {
            // Enter notes editing mode
            enter_notes_edit_mode(state);
        }
        KeyCode::Char('a') => {
            // Add screenshot (when in notes pane or on any test)
            if state.current_test().is_some() {
                state.adding_screenshot = true;
                state.screenshot_input.clear();
                state.focused_pane = FocusedPane::Notes;
            }
        }
        KeyCode::Char('p') => {
            // Only mark status when on header
            if state.focused_pane == FocusedPane::Tests && state.sub_selection == SubSelection::Header {
                if let Some(result) = state.current_result_mut() {
                    result.status = Status::Passed;
                    result.completed_at = Some(chrono::Utc::now().to_rfc3339());
                    state.dirty = true;
                }
            }
        }
        KeyCode::Char('f') => {
            if state.focused_pane == FocusedPane::Tests && state.sub_selection == SubSelection::Header {
                if let Some(result) = state.current_result_mut() {
                    result.status = Status::Failed;
                    result.completed_at = Some(chrono::Utc::now().to_rfc3339());
                    state.dirty = true;
                }
            }
        }
        KeyCode::Char('i') => {
            if state.focused_pane == FocusedPane::Tests && state.sub_selection == SubSelection::Header {
                if let Some(result) = state.current_result_mut() {
                    result.status = Status::Inconclusive;
                    result.completed_at = Some(chrono::Utc::now().to_rfc3339());
                    state.dirty = true;
                }
            }
        }
        KeyCode::Char('s') => {
            if state.focused_pane == FocusedPane::Tests && state.sub_selection == SubSelection::Header {
                if let Some(result) = state.current_result_mut() {
                    result.status = Status::Skipped;
                    result.completed_at = Some(chrono::Utc::now().to_rfc3339());
                    state.dirty = true;
                }
            }
        }
        KeyCode::Char('c') => {
            // Insert suggested command into terminal and focus it
            let cmd = state.current_test().and_then(|t| t.suggested_command.clone());
            if let Some(cmd) = cmd {
                if let Some(ref mut term) = state.terminal {
                    term.send_str(&cmd);
                    state.focused_pane = FocusedPane::Terminal;
                }
            }
        }
        KeyCode::Char('t') => {
            // Toggle theme
            state.theme = state.theme.toggle();
        }
        _ => {}
    }
}

/// Handle keyboard input for the embedded terminal.
fn handle_terminal_input(state: &mut AppState, key: KeyCode, modifiers: KeyModifiers) {
    let Some(ref mut term) = state.terminal else { return };

    match key {
        KeyCode::Char(c) => {
            if modifiers.contains(KeyModifiers::CONTROL) {
                // Send control character
                let ctrl_char = (c as u8).wrapping_sub(b'a').wrapping_add(1);
                term.send_key(&[ctrl_char]);
            } else {
                term.send_char(c);
            }
        }
        KeyCode::Enter => {
            term.send_key(b"\r");
        }
        KeyCode::Backspace => {
            term.send_key(b"\x7f");
        }
        KeyCode::Delete => {
            term.send_key(b"\x1b[3~");
        }
        KeyCode::Up => {
            term.send_key(b"\x1b[A");
        }
        KeyCode::Down => {
            term.send_key(b"\x1b[B");
        }
        KeyCode::Right => {
            term.send_key(b"\x1b[C");
        }
        KeyCode::Left => {
            term.send_key(b"\x1b[D");
        }
        KeyCode::Home => {
            term.send_key(b"\x1b[H");
        }
        KeyCode::End => {
            term.send_key(b"\x1b[F");
        }
        _ => {}
    }
}

/// Enter notes editing mode.
fn enter_notes_edit_mode(state: &mut AppState) {
    if let Some(result) = state.current_result() {
        // Load existing notes into input buffer
        state.notes_input = result.notes.clone().unwrap_or_default();
        state.editing_notes = true;
        state.focused_pane = FocusedPane::Notes;
    }
}

/// Handle keys while editing notes.
fn handle_notes_editing(state: &mut AppState, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            // Save and exit edit mode
            let notes = if state.notes_input.is_empty() {
                None
            } else {
                Some(state.notes_input.clone())
            };
            if let Some(result) = state.current_result_mut() {
                result.notes = notes;
                state.dirty = true;
            }
            state.editing_notes = false;
        }
        KeyCode::Enter => {
            state.notes_input.push('\n');
        }
        KeyCode::Backspace => {
            state.notes_input.pop();
        }
        KeyCode::Char(c) => {
            state.notes_input.push(c);
        }
        _ => {}
    }
}

/// Handle keys while adding screenshot path.
fn handle_screenshot_input(state: &mut AppState, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            // Cancel
            state.adding_screenshot = false;
            state.screenshot_input.clear();
        }
        KeyCode::Enter => {
            // Add the screenshot path
            let path = PathBuf::from(&state.screenshot_input);
            if !state.screenshot_input.is_empty() {
                if let Some(result) = state.current_result_mut() {
                    result.screenshots.push(path);
                    state.dirty = true;
                }
            }
            state.adding_screenshot = false;
            state.screenshot_input.clear();
        }
        KeyCode::Backspace => {
            state.screenshot_input.pop();
        }
        KeyCode::Char(c) => {
            state.screenshot_input.push(c);
        }
        _ => {}
    }
}

/// Navigate down in the tests pane.
fn navigate_down(state: &mut AppState) {
    let Some(test) = state.current_test() else { return };
    let is_expanded = state.expanded_tests.contains(&test.id);

    if !is_expanded {
        // Collapsed: move to next test
        if state.selected_test < state.testlist.tests.len().saturating_sub(1) {
            state.selected_test += 1;
            state.sub_selection = SubSelection::Header;
        }
        return;
    }

    // Expanded: navigate through content
    let setup_count = test.setup.len();
    let verify_count = test.verify.len();

    match state.sub_selection {
        SubSelection::Header => {
            // Move into expanded content
            if setup_count > 0 {
                state.sub_selection = SubSelection::Setup(0);
            } else {
                state.sub_selection = SubSelection::Action;
            }
        }
        SubSelection::Setup(i) => {
            if i + 1 < setup_count {
                state.sub_selection = SubSelection::Setup(i + 1);
            } else {
                state.sub_selection = SubSelection::Action;
            }
        }
        SubSelection::Action => {
            if verify_count > 0 {
                state.sub_selection = SubSelection::Verify(0);
            } else {
                // Move to next test
                if state.selected_test < state.testlist.tests.len().saturating_sub(1) {
                    state.selected_test += 1;
                    state.sub_selection = SubSelection::Header;
                }
            }
        }
        SubSelection::Verify(i) => {
            if i + 1 < verify_count {
                state.sub_selection = SubSelection::Verify(i + 1);
            } else {
                // Move to next test
                if state.selected_test < state.testlist.tests.len().saturating_sub(1) {
                    state.selected_test += 1;
                    state.sub_selection = SubSelection::Header;
                }
            }
        }
    }
}

/// Navigate up in the tests pane.
fn navigate_up(state: &mut AppState) {
    let Some(test) = state.current_test() else { return };
    let is_expanded = state.expanded_tests.contains(&test.id);

    // If on header, move to previous test
    if state.sub_selection == SubSelection::Header {
        if state.selected_test > 0 {
            state.selected_test -= 1;
            // If previous test is expanded, go to its last item
            if let Some(prev_test) = state.current_test() {
                if state.expanded_tests.contains(&prev_test.id) {
                    if !prev_test.verify.is_empty() {
                        state.sub_selection = SubSelection::Verify(prev_test.verify.len() - 1);
                    } else {
                        state.sub_selection = SubSelection::Action;
                    }
                } else {
                    state.sub_selection = SubSelection::Header;
                }
            }
        }
        return;
    }

    if !is_expanded {
        state.sub_selection = SubSelection::Header;
        return;
    }

    // Navigate up through expanded content
    let setup_count = test.setup.len();

    match state.sub_selection {
        SubSelection::Header => unreachable!(),
        SubSelection::Setup(i) => {
            if i > 0 {
                state.sub_selection = SubSelection::Setup(i - 1);
            } else {
                state.sub_selection = SubSelection::Header;
            }
        }
        SubSelection::Action => {
            if setup_count > 0 {
                state.sub_selection = SubSelection::Setup(setup_count - 1);
            } else {
                state.sub_selection = SubSelection::Header;
            }
        }
        SubSelection::Verify(i) => {
            if i > 0 {
                state.sub_selection = SubSelection::Verify(i - 1);
            } else {
                state.sub_selection = SubSelection::Action;
            }
        }
    }
}

/// Toggle a checklist item (setup or verify).
fn toggle_checklist_item(state: &mut AppState) {
    let toggled = match state.sub_selection {
        SubSelection::Setup(i) => {
            if let Some(result) = state.current_result_mut() {
                if let Some(ref mut checked) = result.setup_checked {
                    if let Some(val) = checked.get_mut(i) {
                        *val = !*val;
                        true
                    } else { false }
                } else { false }
            } else { false }
        }
        SubSelection::Verify(i) => {
            if let Some(result) = state.current_result_mut() {
                if let Some(ref mut checked) = result.verify_checked {
                    if let Some(val) = checked.get_mut(i) {
                        *val = !*val;
                        true
                    } else { false }
                } else { false }
            } else { false }
        }
        _ => false,
    };
    if toggled {
        state.dirty = true;
    }
}

fn draw(frame: &mut Frame, state: &AppState) -> LayoutAreas {
    let size = frame.area();

    // Main layout: top area (tests + notes) and bottom (terminal + status)
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),    // Top area
            Constraint::Length(8),  // Terminal placeholder
            Constraint::Length(1),  // Status bar
        ])
        .split(size);

    // Top area: tests (left) and notes (right)
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[0]);

    draw_tests_pane(frame, state, top_chunks[0]);
    draw_notes_pane(frame, state, top_chunks[1]);
    draw_terminal_pane(frame, state, main_chunks[1]);
    draw_status_bar(frame, state, main_chunks[2]);

    // Draw quit confirmation dialog on top if active
    if state.confirm_quit {
        draw_quit_dialog(frame, state, size);
    }

    LayoutAreas {
        tests_pane: top_chunks[0],
        notes_pane: top_chunks[1],
        terminal_pane: main_chunks[1],
    }
}

fn draw_quit_dialog(frame: &mut Frame, state: &AppState, area: Rect) {
    let theme = state.theme;

    // Center a small dialog
    let dialog_width = 40;
    let dialog_height = 5;
    let x = area.width.saturating_sub(dialog_width) / 2;
    let y = area.height.saturating_sub(dialog_height) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    // Clear the area behind the dialog
    frame.render_widget(Clear, dialog_area);

    let text = vec![
        Line::from(""),
        Line::from("You have unsaved changes."),
        Line::from("Quit anyway? (y/n)"),
    ];

    let dialog = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Confirm Quit "),
        )
        .style(Style::default().bg(theme.bg()).fg(theme.fg()));

    frame.render_widget(dialog, dialog_area);
}

fn draw_tests_pane(frame: &mut Frame, state: &AppState, area: Rect) {
    let theme = state.theme;
    let is_focused = state.focused_pane == FocusedPane::Tests;
    let border_style = if is_focused {
        Style::default().fg(theme.accent())
    } else {
        Style::default().fg(theme.dim())
    };

    let selected_style = Style::default()
        .bg(theme.selection_bg())
        .add_modifier(Modifier::BOLD);

    let mut items: Vec<ListItem> = Vec::new();
    let mut selected_line: usize = 0;

    for (i, test) in state.testlist.tests.iter().enumerate() {
        let result = state.results.results.iter().find(|r| r.test_id == test.id);
        let status_icon = match result.map(|r| r.status).unwrap_or(Status::Pending) {
            Status::Pending => "[ ]",
            Status::Passed => "[✓]",
            Status::Failed => "[✗]",
            Status::Inconclusive => "[?]",
            Status::Skipped => "[-]",
        };

        let is_selected_test = i == state.selected_test;
        let is_expanded = state.expanded_tests.contains(&test.id);

        let prefix = if is_expanded { "▼" } else { "▶" };
        let line = format!("{} {} {}", prefix, status_icon, test.title);

        // Track line number of selected item
        if is_selected_test && state.sub_selection == SubSelection::Header {
            selected_line = items.len();
        }

        // Highlight header only if selected test AND sub_selection is Header
        let header_style = if is_selected_test && state.sub_selection == SubSelection::Header {
            selected_style
        } else {
            Style::default()
        };

        items.push(ListItem::new(Line::from(Span::styled(line, header_style))));

        // Show expanded details
        if is_expanded {
            // Setup steps
            if !test.setup.is_empty() {
                items.push(ListItem::new(Line::from("   Setup:")));
                let setup_checked = result.and_then(|r| r.setup_checked.as_ref());
                for (j, step) in test.setup.iter().enumerate() {
                    if is_selected_test && state.sub_selection == SubSelection::Setup(j) {
                        selected_line = items.len();
                    }
                    let checked = setup_checked.and_then(|v| v.get(j)).copied().unwrap_or(false);
                    let check = if checked { "[✓]" } else { "[ ]" };
                    let item_line = format!("     {} {}", check, step);

                    let style = if is_selected_test && state.sub_selection == SubSelection::Setup(j) {
                        selected_style
                    } else {
                        Style::default()
                    };
                    items.push(ListItem::new(Line::from(Span::styled(item_line, style))));
                }
            }

            // Action
            if is_selected_test && state.sub_selection == SubSelection::Action {
                selected_line = items.len();
            }
            let action_line = format!("   Action: {}", test.action);
            let action_style = if is_selected_test && state.sub_selection == SubSelection::Action {
                selected_style
            } else {
                Style::default()
            };
            items.push(ListItem::new(Line::from(Span::styled(action_line, action_style))));

            // Verify steps
            if !test.verify.is_empty() {
                items.push(ListItem::new(Line::from("   Verify:")));
                let verify_checked = result.and_then(|r| r.verify_checked.as_ref());
                for (j, step) in test.verify.iter().enumerate() {
                    if is_selected_test && state.sub_selection == SubSelection::Verify(j) {
                        selected_line = items.len();
                    }
                    let checked = verify_checked
                        .and_then(|v| v.get(j))
                        .copied()
                        .unwrap_or(false);
                    let check = if checked { "[✓]" } else { "[ ]" };
                    let item_line = format!("     {} {}", check, step);

                    let style = if is_selected_test && state.sub_selection == SubSelection::Verify(j) {
                        selected_style
                    } else {
                        Style::default()
                    };
                    items.push(ListItem::new(Line::from(Span::styled(item_line, style))));
                }
            }
        }
    }

    // Calculate visible area (excluding borders)
    let visible_height = area.height.saturating_sub(2) as usize;

    // Apply scroll offset - take a slice of items
    let scroll_offset = state.tests_scroll_offset.min(items.len().saturating_sub(1));
    let visible_items: Vec<ListItem> = items
        .into_iter()
        .skip(scroll_offset)
        .take(visible_height)
        .collect();

    // Show scroll indicator in title if there's more content
    let total_items = scroll_offset + visible_items.len() +
        if scroll_offset + visible_height < selected_line + 1 { 1 } else { 0 };
    let scroll_indicator = if scroll_offset > 0 || total_items > visible_height {
        format!(" [{}-{}] ", scroll_offset + 1, scroll_offset + visible_items.len())
    } else {
        String::new()
    };

    let title = format!(
        " Tests ({}/{}){}",
        state.completed_count(),
        state.testlist.tests.len(),
        scroll_indicator,
    );
    let list = List::new(visible_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title),
    );

    frame.render_widget(list, area);
}

fn draw_notes_pane(frame: &mut Frame, state: &AppState, area: Rect) {
    let theme = state.theme;
    let is_focused = state.focused_pane == FocusedPane::Notes;
    let border_style = if is_focused {
        Style::default().fg(theme.accent())
    } else {
        Style::default().fg(theme.dim())
    };

    // Determine title based on mode
    let title = if state.editing_notes {
        " Notes (EDITING - Esc to save) "
    } else if state.adding_screenshot {
        " Notes (Adding screenshot - Enter to confirm, Esc to cancel) "
    } else {
        " Notes "
    };

    let content = if state.adding_screenshot {
        // Show screenshot input prompt
        vec![
            Line::from("Enter screenshot path:"),
            Line::from(""),
            Line::from(format!("> {}_", state.screenshot_input)),
        ]
    } else if state.editing_notes {
        // Show editable notes content
        let mut lines = Vec::new();
        for line in state.notes_input.lines() {
            lines.push(Line::from(line.to_string()));
        }
        // Add cursor at the end
        if state.notes_input.ends_with('\n') || state.notes_input.is_empty() {
            lines.push(Line::from("_"));
        } else if let Some(last) = lines.last_mut() {
            *last = Line::from(format!("{}_", last.spans.first().map(|s| s.content.as_ref()).unwrap_or("")));
        }
        lines
    } else if let Some(result) = state.current_result() {
        // Normal display mode
        let mut lines = Vec::new();

        if let Some(notes) = &result.notes {
            for line in notes.lines() {
                lines.push(Line::from(line.to_string()));
            }
        } else {
            lines.push(Line::from(Span::styled("(No notes - press 'n' to add)", Style::default().fg(theme.dim()))));
        }

        if !result.screenshots.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from("Screenshots:"));
            for (i, path) in result.screenshots.iter().enumerate() {
                lines.push(Line::from(format!("  [{}] {}", i + 1, path.display())));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("[n] Edit notes  [a] Add screenshot", Style::default().fg(theme.dim()))));

        lines
    } else {
        vec![Line::from("Select a test to view notes")]
    };

    let paragraph = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title),
    );

    frame.render_widget(paragraph, area);
}

fn draw_terminal_pane(frame: &mut Frame, state: &AppState, area: Rect) {
    let theme = state.theme;
    let is_focused = state.focused_pane == FocusedPane::Terminal;
    let border_style = if is_focused {
        Style::default().fg(theme.accent())
    } else {
        Style::default().fg(theme.dim())
    };

    let title = if is_focused {
        " Terminal (Esc to exit, Tab to switch pane) "
    } else {
        " Terminal "
    };

    // Get content from PTY or show placeholder
    let content: Vec<Line> = if let Some(ref term) = state.terminal {
        let screen = term.screen();
        let mut lines = Vec::new();

        // Calculate how many rows we can show (accounting for border)
        let inner_height = area.height.saturating_sub(2);
        let screen_rows = screen.size().0;

        // Get the visible rows from the screen
        for row in 0..inner_height.min(screen_rows) {
            // Get the row contents as a string
            let mut row_str = String::new();
            for col in 0..screen.size().1 {
                let cell = screen.cell(row, col);
                if let Some(cell) = cell {
                    row_str.push(cell.contents().chars().next().unwrap_or(' '));
                } else {
                    row_str.push(' ');
                }
            }
            // Trim trailing spaces
            let text = row_str.trim_end().to_string();
            lines.push(Line::from(text));
        }

        if lines.is_empty() {
            lines.push(Line::from(""));
        }

        lines
    } else {
        // No terminal available - show placeholder with suggested command
        let suggested_cmd = state
            .current_test()
            .and_then(|t| t.suggested_command.as_ref())
            .map(|s| format!("Suggested: {}", s))
            .unwrap_or_else(|| "(No suggested command)".to_string());

        vec![
            Line::from("Terminal not available"),
            Line::from(""),
            Line::from(suggested_cmd),
        ]
    };

    let paragraph = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title),
    );

    frame.render_widget(paragraph, area);

    // Show cursor when terminal is focused
    if is_focused {
        if let Some(ref term) = state.terminal {
            let screen = term.screen();
            let cursor_pos = screen.cursor_position();
            // Add 1 for the border offset
            let cursor_x = area.x + 1 + cursor_pos.1;
            let cursor_y = area.y + 1 + cursor_pos.0;
            // Only show cursor if within the pane
            if cursor_x < area.x + area.width - 1 && cursor_y < area.y + area.height - 1 {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }
}

fn draw_status_bar(frame: &mut Frame, state: &AppState, area: Rect) {
    let theme = state.theme;
    let test_name = state
        .current_test()
        .map(|t| t.title.as_str())
        .unwrap_or("No test selected");

    // Show different actions based on mode
    let status = if state.editing_notes {
        " EDITING NOTES │ [Esc] Save and exit │ Type to edit ".to_string()
    } else if state.adding_screenshot {
        " ADDING SCREENSHOT │ [Enter] Confirm │ [Esc] Cancel │ Type path ".to_string()
    } else {
        let actions = match state.sub_selection {
            SubSelection::Header => "[P]ass [F]ail [I]nconclusive [S]kip │ [Enter] Expand",
            SubSelection::Setup(_) | SubSelection::Verify(_) => "[Space] Toggle",
            SubSelection::Action => "Action (read-only)",
        };
        // Show [c] hint if current test has a suggested command
        let cmd_hint = if state.current_test().and_then(|t| t.suggested_command.as_ref()).is_some() {
            "[c] Run cmd "
        } else {
            ""
        };
        format!(
            " {} │ [n] Notes [a] Screenshot {}│ [t] Theme │ [Tab] Pane │ [Q]uit │ {} ",
            actions, cmd_hint, test_name
        )
    };

    let paragraph = Paragraph::new(Line::from(status))
        .style(Style::default().bg(theme.selection_bg()).fg(theme.fg()));

    frame.render_widget(paragraph, area);
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
        // Dark and light themes should have different colors
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
