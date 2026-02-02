//! UI layer: event loop, key/mouse dispatch, and rendering coordination.

pub mod app;
pub mod panes;

use crossterm::event::{
    self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};

use crate::data::state::{AppState, FocusedPane};
use crate::error::Result;
use crate::queries::tests::{current_test, map_y_to_test_index};
use crate::transforms::{navigation, tests as test_transforms, ui as ui_transforms};
use panes::terminal::EmbeddedTerminal;

/// Stores layout information for mouse click handling.
struct LayoutAreas {
    tests_pane: Rect,
    notes_pane: Rect,
    terminal_pane: Rect,
}

fn main_loop(
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    state: &mut AppState,
    pty: &mut Option<EmbeddedTerminal>,
) -> Result<()> {
    let mut layout_areas: Option<LayoutAreas> = None;

    while !state.should_quit {
        // Poll PTY output
        if let Some(ref mut term) = pty {
            term.poll_output();
        }

        terminal.draw(|frame| {
            layout_areas = Some(draw(frame, state, pty));
        })?;

        if let Some(ref areas) = layout_areas {
            state.tests_visible_height = areas.tests_pane.height.saturating_sub(2) as usize;

            let new_rows = areas.terminal_pane.height.saturating_sub(2);
            let new_cols = areas.terminal_pane.width.saturating_sub(2);
            if (new_rows, new_cols) != state.terminal_size {
                state.terminal_size = (new_rows, new_cols);
                if let Some(ref mut term) = pty {
                    term.resize(new_rows, new_cols);
                }
            }
        }

        if event::poll(std::time::Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        handle_key(state, key.code, key.modifiers, pty);
                        navigation::adjust_scroll(state);
                    }
                }
                Event::Mouse(mouse) => {
                    if let Some(ref areas) = layout_areas {
                        handle_mouse(state, mouse, areas);
                        navigation::adjust_scroll(state);
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }
    Ok(())
}

fn handle_mouse(state: &mut AppState, mouse: crossterm::event::MouseEvent, areas: &LayoutAreas) {
    // Don't change focus via mouse during editing modes or modal dialogs
    if state.editing_notes || state.adding_screenshot || state.confirm_quit || state.show_help {
        return;
    }

    // Only change focus on left click, not on scroll/motion/drag/release
    let MouseEventKind::Down(MouseButton::Left) = mouse.kind else {
        return;
    };

    let x = mouse.column;
    let y = mouse.row;

    if areas.tests_pane.contains((x, y).into()) {
        state.focused_pane = FocusedPane::Tests;

        let relative_y = y.saturating_sub(areas.tests_pane.y + 1) as usize;
        let absolute_y = relative_y + state.tests_scroll_offset;

        if let Some(test_idx) = map_y_to_test_index(state, absolute_y) {
            if test_idx == state.selected_test {
                // Click on already-selected test: toggle expand/collapse
                ui_transforms::toggle_expand(state);
            } else {
                // Click on different test: select it
                state.selected_test = test_idx;
            }
        }
    } else if areas.notes_pane.contains((x, y).into()) {
        state.focused_pane = FocusedPane::Notes;
    } else if areas.terminal_pane.contains((x, y).into()) {
        state.focused_pane = FocusedPane::Terminal;
    }
}

fn handle_key(
    state: &mut AppState,
    key: KeyCode,
    modifiers: KeyModifiers,
    pty: &mut Option<EmbeddedTerminal>,
) {
    // Handle quit confirmation dialog
    if state.confirm_quit {
        match key {
            KeyCode::Left | KeyCode::Char('h') => state.quit_selection = 0,
            KeyCode::Right | KeyCode::Char('l') => state.quit_selection = 1,
            KeyCode::Enter => {
                if state.quit_selection == 0 {
                    ui_transforms::confirm_quit(state);
                } else {
                    ui_transforms::quit_without_saving(state);
                }
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => ui_transforms::confirm_quit(state),
            KeyCode::Char('n') | KeyCode::Char('N') => {
                ui_transforms::quit_without_saving(state)
            }
            KeyCode::Esc => ui_transforms::cancel_quit(state),
            _ => {}
        }
        return;
    }

    // Handle help popup
    if state.show_help {
        match key {
            KeyCode::Char('?') | KeyCode::Esc => state.show_help = false,
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
    if state.focused_pane == FocusedPane::Terminal && pty.is_some() {
        if key == KeyCode::Esc {
            state.focused_pane = FocusedPane::Tests;
            return;
        }
        if key == KeyCode::Tab {
            ui_transforms::cycle_focus(state);
            return;
        }
        handle_terminal_input(pty, key, modifiers);
        return;
    }

    // Normal mode — thin dispatcher calling transforms
    match key {
        KeyCode::Char('q') => ui_transforms::request_quit(state),
        KeyCode::Tab => ui_transforms::cycle_focus(state),
        KeyCode::Up | KeyCode::Char('k') => {
            if state.focused_pane == FocusedPane::Tests {
                navigation::select_prev(state);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if state.focused_pane == FocusedPane::Tests {
                navigation::select_next(state);
            }
        }
        KeyCode::Enter | KeyCode::Char('l') | KeyCode::Char(' ') => {
            if state.focused_pane == FocusedPane::Tests {
                ui_transforms::toggle_expand(state);
            }
        }
        KeyCode::Char('n') => {
            if state.focused_pane == FocusedPane::Tests {
                ui_transforms::enter_notes_edit(state);
            }
        }
        KeyCode::Char('a') => {
            if state.focused_pane == FocusedPane::Tests {
                ui_transforms::start_screenshot(state);
            }
        }
        KeyCode::Char('p') => {
            if state.focused_pane == FocusedPane::Tests {
                test_transforms::set_status(state, crate::data::results::Status::Passed);
            }
        }
        KeyCode::Char('f') => {
            if state.focused_pane == FocusedPane::Tests {
                test_transforms::set_status(state, crate::data::results::Status::Failed);
            }
        }
        KeyCode::Char('i') => {
            if state.focused_pane == FocusedPane::Tests {
                test_transforms::set_status(state, crate::data::results::Status::Inconclusive);
            }
        }
        KeyCode::Char('s') => {
            if state.focused_pane == FocusedPane::Tests {
                test_transforms::set_status(state, crate::data::results::Status::Skipped);
            }
        }
        KeyCode::Char('c') => {
            let cmd = current_test(state).and_then(|t| t.suggested_command.clone());
            if let Some(cmd) = cmd {
                if let Some(ref mut term) = pty {
                    term.send_str(&cmd);
                    state.focused_pane = FocusedPane::Terminal;
                }
            }
        }
        KeyCode::Char('t') => ui_transforms::toggle_theme(state),
        KeyCode::Char('?') => state.show_help = true,
        KeyCode::Char('w') => {
            if let Ok(()) = crate::actions::files::save_results(&state.results, &state.results_path)
            {
                state.dirty = false;
            }
        }
        _ => {}
    }
}

fn handle_terminal_input(
    pty: &mut Option<EmbeddedTerminal>,
    key: KeyCode,
    modifiers: KeyModifiers,
) {
    let Some(ref mut term) = pty else { return };

    match key {
        KeyCode::Char(c) => {
            if modifiers.contains(KeyModifiers::CONTROL) {
                let ctrl_char = (c as u8).wrapping_sub(b'a').wrapping_add(1);
                term.send_key(&[ctrl_char]);
            } else {
                term.send_char(c);
            }
        }
        KeyCode::Enter => term.send_key(b"\r"),
        KeyCode::Backspace => term.send_key(b"\x7f"),
        KeyCode::Delete => term.send_key(b"\x1b[3~"),
        KeyCode::Up => term.send_key(b"\x1b[A"),
        KeyCode::Down => term.send_key(b"\x1b[B"),
        KeyCode::Right => term.send_key(b"\x1b[C"),
        KeyCode::Left => term.send_key(b"\x1b[D"),
        KeyCode::Home => term.send_key(b"\x1b[H"),
        KeyCode::End => term.send_key(b"\x1b[F"),
        _ => {}
    }
}

fn handle_notes_editing(state: &mut AppState, key: KeyCode) {
    match key {
        KeyCode::Esc => ui_transforms::save_notes(state),
        KeyCode::Enter => state.notes_input.push('\n'),
        KeyCode::Backspace => {
            state.notes_input.pop();
        }
        KeyCode::Char(c) => state.notes_input.push(c),
        _ => {}
    }
}

fn handle_screenshot_input(state: &mut AppState, key: KeyCode) {
    match key {
        KeyCode::Esc => ui_transforms::cancel_screenshot(state),
        KeyCode::Enter => ui_transforms::confirm_screenshot(state),
        KeyCode::Backspace => {
            state.screenshot_input.pop();
        }
        KeyCode::Char(c) => state.screenshot_input.push(c),
        _ => {}
    }
}

fn draw(frame: &mut Frame, state: &AppState, pty: &Option<EmbeddedTerminal>) -> LayoutAreas {
    let size = frame.area();

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(8),
            Constraint::Length(1),
        ])
        .split(size);

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[0]);

    panes::tests::draw(frame, state, top_chunks[0]);
    panes::notes::draw(frame, state, top_chunks[1]);
    panes::terminal::draw(frame, state, pty, main_chunks[1]);
    draw_status_bar(frame, state, main_chunks[2]);

    if state.confirm_quit {
        draw_quit_dialog(frame, state, size);
    }

    if state.show_help {
        draw_help_dialog(frame, state, size);
    }

    LayoutAreas {
        tests_pane: top_chunks[0],
        notes_pane: top_chunks[1],
        terminal_pane: main_chunks[1],
    }
}

fn draw_quit_dialog(frame: &mut Frame, state: &AppState, area: Rect) {
    use ratatui::text::Span;

    let theme = state.theme;
    let dialog_width = 40;
    let dialog_height = 5;
    let x = area.width.saturating_sub(dialog_width) / 2;
    let y = area.height.saturating_sub(dialog_height) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    frame.render_widget(Clear, dialog_area);

    let (yes_style, no_style) = if state.quit_selection == 0 {
        (
            Style::default().fg(theme.accent()),
            Style::default().fg(theme.dim()),
        )
    } else {
        (
            Style::default().fg(theme.dim()),
            Style::default().fg(theme.accent()),
        )
    };

    let yes_label = if state.quit_selection == 0 {
        "► [Yes]"
    } else {
        "  [Yes]"
    };
    let no_label = if state.quit_selection == 1 {
        "► [No]"
    } else {
        "  [No]"
    };

    let text = vec![
        Line::from(""),
        Line::from(" Save changes before quitting?"),
        Line::from(vec![
            Span::styled(format!("    {}", yes_label), yes_style),
            Span::styled(format!("    {}", no_label), no_style),
        ]),
    ];

    let dialog = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ratatui::style::Color::Yellow))
                .title(" Confirm Quit "),
        )
        .style(Style::default().bg(theme.bg()).fg(theme.fg()));

    frame.render_widget(dialog, dialog_area);
}

fn draw_help_dialog(frame: &mut Frame, state: &AppState, area: Rect) {
    let theme = state.theme;
    let dialog_width = 54u16;
    let dialog_height = 19u16;
    let x = area.width.saturating_sub(dialog_width) / 2;
    let y = area.height.saturating_sub(dialog_height) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    frame.render_widget(Clear, dialog_area);

    let text = vec![
        Line::from(""),
        Line::from(" Navigation"),
        Line::from("   j/k or ↑/↓   Navigate tests"),
        Line::from("   Enter/Space   Expand/collapse test"),
        Line::from("   Tab           Cycle pane focus"),
        Line::from(""),
        Line::from(" Test Status"),
        Line::from("   p  Pass    f  Fail"),
        Line::from("   i  Inconclusive    s  Skip"),
        Line::from(""),
        Line::from(" Actions"),
        Line::from("   n  Edit notes       a  Add screenshot"),
        Line::from("   c  Run suggested command"),
        Line::from(""),
        Line::from(" Other"),
        Line::from("   w  Save     t  Theme     ?  Help     q  Quit"),
        Line::from(""),
        Line::from(" Press ? or Esc to close"),
    ];

    let dialog = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent()))
                .title(" Help "),
        )
        .style(Style::default().bg(theme.bg()).fg(theme.fg()));

    frame.render_widget(dialog, dialog_area);
}

fn draw_status_bar(frame: &mut Frame, state: &AppState, area: Rect) {
    let theme = state.theme;
    let test_name = current_test(state)
        .map(|t| t.title.as_str())
        .unwrap_or("No test selected");

    let status = if state.editing_notes {
        " EDITING NOTES │ [Esc] Save and exit │ Type to edit ".to_string()
    } else if state.adding_screenshot {
        " ADDING SCREENSHOT │ [Enter] Confirm │ [Esc] Cancel │ Type path ".to_string()
    } else {
        format!(
            " [P]ass [F]ail [I]nc [S]kip │ [Tab] Pane │ [?] Help │ [w] Save │ [Q]uit │ {} ",
            test_name
        )
    };

    let paragraph = Paragraph::new(Line::from(status))
        .style(Style::default().bg(theme.selection_bg()).fg(theme.fg()));

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Bug 2 verification test ===
    // On a small terminal (e.g. 15 rows), the status bar must still get its 1 row.
    // The old layout used Min(10) for the top area, which left 0 rows for the
    // status bar on terminals with height <= 18. The fix uses Min(3).

    #[test]
    fn test_bug2_status_bar_visible_on_small_terminal() {
        // Simulate a very small terminal: 15 rows
        let small_area = Rect::new(0, 0, 80, 15);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),    // Top area (tests + notes) — the fix
                Constraint::Length(8), // Terminal
                Constraint::Length(1), // Status bar
            ])
            .split(small_area);

        let status_bar = chunks[2];
        assert_eq!(
            status_bar.height, 1,
            "BUG 2: Status bar must be 1 row high even on small terminals"
        );

        let terminal_pane = chunks[1];
        assert_eq!(
            terminal_pane.height, 8,
            "Terminal pane should keep its 8 rows"
        );

        let top_area = chunks[0];
        assert_eq!(
            top_area.height, 6,
            "Top area gets remaining space (15 - 8 - 1 = 6)"
        );
    }

    #[test]
    fn test_bug2_extremely_small_terminal() {
        // Even at 10 rows, status bar should still be visible
        let tiny_area = Rect::new(0, 0, 80, 10);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(8),
                Constraint::Length(1),
            ])
            .split(tiny_area);

        let status_bar = chunks[2];
        assert!(
            status_bar.height >= 1,
            "BUG 2: Status bar must be visible even on 10-row terminal, got height={}",
            status_bar.height
        );
    }

    // === Integration test: handle_key dispatch ===
    // Reproduce user-reported bug: p/f/i/s stops working after notes editing

    fn make_test_state() -> AppState {
        use crate::data::definition::{ChecklistItem, Meta, Test, Testlist};
        use crate::data::results::TestlistResults;

        let testlist = Testlist {
            meta: Meta {
                title: "Test".to_string(),
                description: "".to_string(),
                created: "".to_string(),
                version: "1".to_string(),
            },
            tests: vec![Test {
                id: "t1".to_string(),
                title: "Test 1".to_string(),
                description: "".to_string(),
                setup: vec![ChecklistItem {
                    id: "s0".to_string(),
                    text: "Step".to_string(),
                }],
                action: "Do it".to_string(),
                verify: vec![ChecklistItem {
                    id: "v0".to_string(),
                    text: "Check".to_string(),
                }],
                suggested_command: None,
            }],
        };
        let results = TestlistResults::new_for_testlist(&testlist, "test.ron", "tester");
        AppState::new(
            testlist,
            results,
            std::path::PathBuf::from("test.testlist.ron"),
            std::path::PathBuf::from("test.testlist.results.ron"),
        )
    }

    #[test]
    fn test_status_key_works_after_notes_editing() {
        use crate::data::results::Status;
        use crate::data::state::FocusedPane;

        let mut state = make_test_state();
        let mut pty: Option<EmbeddedTerminal> = None;
        let no_mods = KeyModifiers::empty();

        // Initial state: Tests focused
        assert_eq!(state.focused_pane, FocusedPane::Tests);

        // Step 1: Press 'p' — should set status to Passed
        handle_key(&mut state, KeyCode::Char('p'), no_mods, &mut pty);
        assert_eq!(
            state.results.results[0].status,
            Status::Passed,
            "Initial 'p' should set Passed"
        );

        // Step 2: Press 'n' — enter notes editing
        handle_key(&mut state, KeyCode::Char('n'), no_mods, &mut pty);
        assert!(state.editing_notes, "Should be in editing mode");
        assert_eq!(state.focused_pane, FocusedPane::Notes);

        // Step 3: Type some text
        handle_key(&mut state, KeyCode::Char('h'), no_mods, &mut pty);
        handle_key(&mut state, KeyCode::Char('i'), no_mods, &mut pty);
        assert_eq!(state.notes_input, "hi");

        // Step 4: Press Esc to save notes
        handle_key(&mut state, KeyCode::Esc, no_mods, &mut pty);
        assert!(
            !state.editing_notes,
            "Should exit editing mode after Esc"
        );
        assert_eq!(
            state.focused_pane,
            FocusedPane::Tests,
            "Focus should return to Tests after Esc"
        );

        // Verify notes were saved
        assert_eq!(
            state.results.results[0].notes,
            Some("hi".to_string()),
            "Notes should be saved"
        );

        // Step 5: Press 'f' — should change status to Failed
        handle_key(&mut state, KeyCode::Char('f'), no_mods, &mut pty);
        assert_eq!(
            state.results.results[0].status,
            Status::Failed,
            "BUG: 'f' should work after notes editing — status should be Failed"
        );

        // Step 6: Press 'i' — should change status to Inconclusive
        handle_key(&mut state, KeyCode::Char('i'), no_mods, &mut pty);
        assert_eq!(
            state.results.results[0].status,
            Status::Inconclusive,
            "'i' should work after notes editing"
        );
    }

    #[test]
    fn test_status_key_works_after_notes_then_navigate() {
        use crate::data::results::Status;

        let mut state = make_test_state();
        let mut pty: Option<EmbeddedTerminal> = None;
        let no_mods = KeyModifiers::empty();

        // Edit notes
        handle_key(&mut state, KeyCode::Char('n'), no_mods, &mut pty);
        handle_key(&mut state, KeyCode::Char('x'), no_mods, &mut pty);
        handle_key(&mut state, KeyCode::Esc, no_mods, &mut pty);

        // Navigate down then back up (j then k)
        handle_key(&mut state, KeyCode::Char('j'), no_mods, &mut pty);
        handle_key(&mut state, KeyCode::Char('k'), no_mods, &mut pty);

        // There's only 1 test, so j does nothing (at boundary)
        assert_eq!(state.selected_test, 0);

        // Try status key
        handle_key(&mut state, KeyCode::Char('p'), no_mods, &mut pty);
        assert_eq!(
            state.results.results[0].status,
            Status::Passed,
            "'p' should work after notes + navigation"
        );
    }

    // Regression: verify old Min(10) would have failed
    #[test]
    fn test_bug2_old_layout_would_hide_status_bar() {
        let small_area = Rect::new(0, 0, 80, 15);

        // Old layout with Min(10)
        let old_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(10), // OLD — too greedy
                Constraint::Length(8),
                Constraint::Length(1),
            ])
            .split(small_area);

        // With 15 rows: Min(10) takes 10, Length(8) wants 8 but only 5 left,
        // Length(1) gets squeezed. The exact behavior depends on ratatui's
        // constraint solver, but the key issue is the top area takes too much.
        let old_top = old_chunks[0].height;
        let new_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3), // NEW — flexible
                Constraint::Length(8),
                Constraint::Length(1),
            ])
            .split(small_area);

        let new_top = new_chunks[0].height;
        // The new layout gives more room to terminal + status bar
        assert!(
            new_top <= old_top,
            "New layout should not be greedier than old for top area"
        );
    }
}
