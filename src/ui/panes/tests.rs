//! Tests pane rendering.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::data::results::ChecklistSection;
use crate::data::state::{AppState, FocusedPane, SubSelection};
use crate::queries::checklist::is_checked;
use crate::queries::tests::{completed_count, result_for_test};

/// Draw the tests pane.
pub fn draw(frame: &mut Frame, state: &AppState, area: Rect) {
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

    for (i, test) in state.testlist.tests.iter().enumerate() {
        let result = result_for_test(&state.results, &test.id);
        let status = result.map(|r| r.status).unwrap_or_default();
        let status_icon = match status {
            crate::data::results::Status::Pending => "[ ]",
            crate::data::results::Status::Passed => "[✓]",
            crate::data::results::Status::Failed => "[✗]",
            crate::data::results::Status::Inconclusive => "[?]",
            crate::data::results::Status::Skipped => "[-]",
        };

        let is_selected_test = i == state.selected_test;
        let is_expanded = state.expanded_tests.contains(&test.id);

        let prefix = if is_expanded { "▼" } else { "▶" };
        let line = format!("{} {} {}", prefix, status_icon, test.title);

        let header_style = if is_selected_test && state.sub_selection == SubSelection::Header {
            selected_style
        } else {
            Style::default()
        };

        items.push(ListItem::new(Line::from(Span::styled(line, header_style))));

        if is_expanded {
            // Setup steps
            if !test.setup.is_empty() {
                items.push(ListItem::new(Line::from("   Setup:")));
                for (j, item) in test.setup.iter().enumerate() {
                    let checked =
                        is_checked(&state.results, &test.id, ChecklistSection::Setup, &item.id);
                    let check = if checked { "[✓]" } else { "[ ]" };
                    let item_line = format!("     {} {}", check, item.text);

                    let style = if is_selected_test && state.sub_selection == SubSelection::Setup(j)
                    {
                        selected_style
                    } else {
                        Style::default()
                    };
                    items.push(ListItem::new(Line::from(Span::styled(item_line, style))));
                }
            }

            // Action
            let action_line = format!("   Action: {}", test.action);
            let action_style = if is_selected_test && state.sub_selection == SubSelection::Action {
                selected_style
            } else {
                Style::default()
            };
            items.push(ListItem::new(Line::from(Span::styled(
                action_line,
                action_style,
            ))));

            // Verify steps
            if !test.verify.is_empty() {
                items.push(ListItem::new(Line::from("   Verify:")));
                for (j, item) in test.verify.iter().enumerate() {
                    let checked =
                        is_checked(&state.results, &test.id, ChecklistSection::Verify, &item.id);
                    let check = if checked { "[✓]" } else { "[ ]" };
                    let item_line = format!("     {} {}", check, item.text);

                    let style =
                        if is_selected_test && state.sub_selection == SubSelection::Verify(j) {
                            selected_style
                        } else {
                            Style::default()
                        };
                    items.push(ListItem::new(Line::from(Span::styled(item_line, style))));
                }
            }
        }
    }

    let visible_height = area.height.saturating_sub(2) as usize;
    let scroll_offset = state.tests_scroll_offset.min(items.len().saturating_sub(1));
    let visible_items: Vec<ListItem> = items
        .into_iter()
        .skip(scroll_offset)
        .take(visible_height)
        .collect();

    let scroll_indicator = if scroll_offset > 0
        || scroll_offset + visible_height < scroll_offset + visible_items.len() + 1
    {
        if !visible_items.is_empty() {
            format!(
                " [{}-{}] ",
                scroll_offset + 1,
                scroll_offset + visible_items.len()
            )
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let title = format!(
        " Tests ({}/{}){}",
        completed_count(state),
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
