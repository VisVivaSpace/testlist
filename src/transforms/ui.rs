//! Transforms for UI state changes.

use crate::data::state::{AppState, FocusedPane};
use crate::queries::tests::current_result;

/// Cycle focus to the next pane.
pub fn cycle_focus(state: &mut AppState) {
    state.focused_pane = state.focused_pane.next();
}

/// Enter notes editing mode.
pub fn enter_notes_edit(state: &mut AppState) {
    if let Some(result) = current_result(state) {
        state.notes_input = result.notes.clone().unwrap_or_default();
        state.editing_notes = true;
        state.focused_pane = FocusedPane::Notes;
    }
}

/// Save notes and exit editing mode.
pub fn save_notes(state: &mut AppState) {
    let notes = if state.notes_input.is_empty() {
        None
    } else {
        Some(state.notes_input.clone())
    };
    let test_id = state
        .testlist
        .tests
        .get(state.selected_test)
        .map(|t| t.id.clone());
    if let Some(test_id) = test_id {
        if let Some(result) = state.results.get_result_mut(&test_id) {
            result.notes = notes;
            state.dirty = true;
        }
    }
    state.editing_notes = false;
    state.focused_pane = FocusedPane::Tests;
}

/// Start adding a screenshot.
pub fn start_screenshot(state: &mut AppState) {
    if state.testlist.tests.get(state.selected_test).is_some() {
        state.adding_screenshot = true;
        state.screenshot_input.clear();
        state.focused_pane = FocusedPane::Notes;
    }
}

/// Cancel screenshot input.
pub fn cancel_screenshot(state: &mut AppState) {
    state.adding_screenshot = false;
    state.screenshot_input.clear();
    state.focused_pane = FocusedPane::Tests;
}

/// Confirm screenshot input.
pub fn confirm_screenshot(state: &mut AppState) {
    if !state.screenshot_input.is_empty() {
        let path = std::path::PathBuf::from(&state.screenshot_input);
        let test_id = state
            .testlist
            .tests
            .get(state.selected_test)
            .map(|t| t.id.clone());
        if let Some(test_id) = test_id {
            if let Some(result) = state.results.get_result_mut(&test_id) {
                result.screenshots.push(path);
                state.dirty = true;
            }
        }
    }
    state.adding_screenshot = false;
    state.screenshot_input.clear();
    state.focused_pane = FocusedPane::Tests;
}

/// Toggle theme between dark and light.
pub fn toggle_theme(state: &mut AppState) {
    state.theme = state.theme.toggle();
}

/// Toggle expand/collapse on the currently selected test header.
pub fn toggle_expand(state: &mut AppState) {
    if let Some(test) = state.testlist.tests.get(state.selected_test) {
        let id = test.id.clone();
        if state.expanded_tests.contains(&id) {
            state.expanded_tests.remove(&id);
        } else {
            state.expanded_tests.insert(id);
        }
    }
}

/// Request quit — shows confirmation if dirty.
pub fn request_quit(state: &mut AppState) {
    if state.dirty {
        state.confirm_quit = true;
    } else {
        state.should_quit = true;
    }
}

/// Confirm quit (from dialog).
pub fn confirm_quit(state: &mut AppState) {
    state.should_quit = true;
}

/// Cancel quit (from dialog).
pub fn cancel_quit(state: &mut AppState) {
    state.confirm_quit = false;
}
