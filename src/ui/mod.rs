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

use crate::data::state::{AppState, FocusedPane, SubSelection};
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
    let x = mouse.column;
    let y = mouse.row;

    if areas.tests_pane.contains((x, y).into()) {
        state.focused_pane = FocusedPane::Tests;

        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
            let relative_y = y.saturating_sub(areas.tests_pane.y + 1) as usize;
            let absolute_y = relative_y + state.tests_scroll_offset;

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

fn handle_key(
    state: &mut AppState,
    key: KeyCode,
    modifiers: KeyModifiers,
    pty: &mut Option<EmbeddedTerminal>,
) {
    // Handle quit confirmation dialog
    if state.confirm_quit {
        match key {
            KeyCode::Char('y') | KeyCode::Char('Y') => ui_transforms::confirm_quit(state),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                ui_transforms::cancel_quit(state)
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
        KeyCode::Enter | KeyCode::Char('l') => {
            if state.focused_pane == FocusedPane::Tests
                && state.sub_selection == SubSelection::Header
            {
                ui_transforms::toggle_expand(state);
            }
        }
        KeyCode::Char(' ') => {
            if state.focused_pane == FocusedPane::Tests {
                test_transforms::toggle_checklist(state);
            }
        }
        KeyCode::Char('n') => ui_transforms::enter_notes_edit(state),
        KeyCode::Char('a') => ui_transforms::start_screenshot(state),
        KeyCode::Char('p') => {
            if state.focused_pane == FocusedPane::Tests
                && state.sub_selection == SubSelection::Header
            {
                test_transforms::set_status(state, crate::data::results::Status::Passed);
            }
        }
        KeyCode::Char('f') => {
            if state.focused_pane == FocusedPane::Tests
                && state.sub_selection == SubSelection::Header
            {
                test_transforms::set_status(state, crate::data::results::Status::Failed);
            }
        }
        KeyCode::Char('i') => {
            if state.focused_pane == FocusedPane::Tests
                && state.sub_selection == SubSelection::Header
            {
                test_transforms::set_status(state, crate::data::results::Status::Inconclusive);
            }
        }
        KeyCode::Char('s') => {
            if state.focused_pane == FocusedPane::Tests
                && state.sub_selection == SubSelection::Header
            {
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

    LayoutAreas {
        tests_pane: top_chunks[0],
        notes_pane: top_chunks[1],
        terminal_pane: main_chunks[1],
    }
}

fn draw_quit_dialog(frame: &mut Frame, state: &AppState, area: Rect) {
    let theme = state.theme;
    let dialog_width = 40;
    let dialog_height = 5;
    let x = area.width.saturating_sub(dialog_width) / 2;
    let y = area.height.saturating_sub(dialog_height) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

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
                .border_style(Style::default().fg(ratatui::style::Color::Yellow))
                .title(" Confirm Quit "),
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
        let actions = match state.sub_selection {
            SubSelection::Header => "[P]ass [F]ail [I]nconclusive [S]kip │ [Enter] Expand",
            SubSelection::Setup(_) | SubSelection::Verify(_) => "[Space] Toggle",
            SubSelection::Action => "Action (read-only)",
        };
        let cmd_hint = if current_test(state)
            .and_then(|t| t.suggested_command.as_ref())
            .is_some()
        {
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
