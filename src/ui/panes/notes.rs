//! Notes pane rendering.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::data::state::{AppState, FocusedPane};
use crate::queries::tests::current_result;

/// Draw the notes pane.
pub fn draw(frame: &mut Frame, state: &AppState, area: Rect) {
    let theme = state.theme;
    let is_focused = state.focused_pane == FocusedPane::Notes;
    let border_style = if is_focused {
        Style::default().fg(theme.accent())
    } else {
        Style::default().fg(theme.dim())
    };

    let title = if state.editing_notes {
        " Notes (EDITING - Esc to save) "
    } else if state.adding_screenshot {
        " Notes (Adding screenshot - Enter to confirm, Esc to cancel) "
    } else {
        " Notes "
    };

    let content = if state.adding_screenshot {
        vec![
            Line::from("Enter screenshot path:"),
            Line::from(""),
            Line::from(format!("> {}_", state.screenshot_input)),
        ]
    } else if state.editing_notes {
        let mut lines = Vec::new();
        for line in state.notes_input.lines() {
            lines.push(Line::from(line.to_string()));
        }
        if state.notes_input.ends_with('\n') || state.notes_input.is_empty() {
            lines.push(Line::from("_"));
        } else if let Some(last) = lines.last_mut() {
            *last = Line::from(format!(
                "{}_",
                last.spans.first().map(|s| s.content.as_ref()).unwrap_or("")
            ));
        }
        lines
    } else if let Some(result) = current_result(state) {
        let mut lines = Vec::new();

        if let Some(notes) = &result.notes {
            for line in notes.lines() {
                lines.push(Line::from(line.to_string()));
            }
        } else {
            lines.push(Line::from(Span::styled(
                "(No notes - press 'n' to add)",
                Style::default().fg(theme.dim()),
            )));
        }

        if !result.screenshots.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from("Screenshots:"));
            for (i, path) in result.screenshots.iter().enumerate() {
                lines.push(Line::from(format!("  [{}] {}", i + 1, path.display())));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "[n] Edit notes  [a] Add screenshot",
            Style::default().fg(theme.dim()),
        )));

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
