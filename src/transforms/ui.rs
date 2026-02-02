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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::definition::{ChecklistItem, Meta, Test, Testlist};
    use crate::data::results::{Status, TestlistResults};
    use crate::data::state::SubSelection;
    use crate::transforms::tests::set_status;

    fn make_state() -> AppState {
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

    // === Bug 1 verification tests ===
    // After editing notes (n -> type -> Esc), focus must return to Tests pane
    // so that status keys (p/f/i/s) work immediately.

    #[test]
    fn test_bug1_notes_edit_then_status_key() {
        let mut state = make_state();
        assert_eq!(state.focused_pane, FocusedPane::Tests);
        assert_eq!(state.sub_selection, SubSelection::Header);

        // User presses 'n' to edit notes
        enter_notes_edit(&mut state);
        assert_eq!(state.focused_pane, FocusedPane::Notes);
        assert!(state.editing_notes);

        // User types some notes
        state.notes_input.push_str("looks good");

        // User presses Esc to save
        save_notes(&mut state);
        assert!(!state.editing_notes);
        assert_eq!(
            state.focused_pane,
            FocusedPane::Tests,
            "BUG 1: Focus must return to Tests after notes Esc"
        );

        // Now user presses 'p' — this should work because focus is Tests + Header
        assert_eq!(state.sub_selection, SubSelection::Header);
        set_status(&mut state, Status::Passed);
        assert_eq!(
            state.results.results[0].status,
            Status::Passed,
            "BUG 1: Status key must work after exiting notes"
        );
    }

    #[test]
    fn test_bug1_screenshot_cancel_then_status_key() {
        let mut state = make_state();

        // User presses 'a' to add screenshot
        start_screenshot(&mut state);
        assert_eq!(state.focused_pane, FocusedPane::Notes);
        assert!(state.adding_screenshot);

        // User presses Esc to cancel
        cancel_screenshot(&mut state);
        assert!(!state.adding_screenshot);
        assert_eq!(
            state.focused_pane,
            FocusedPane::Tests,
            "BUG 1: Focus must return to Tests after screenshot Esc"
        );

        // Status key should work
        set_status(&mut state, Status::Failed);
        assert_eq!(state.results.results[0].status, Status::Failed);
    }

    #[test]
    fn test_bug1_screenshot_confirm_then_status_key() {
        let mut state = make_state();

        // User presses 'a' to add screenshot
        start_screenshot(&mut state);
        state.screenshot_input = "/tmp/screenshot.png".to_string();

        // User presses Enter to confirm
        confirm_screenshot(&mut state);
        assert!(!state.adding_screenshot);
        assert_eq!(
            state.focused_pane,
            FocusedPane::Tests,
            "BUG 1: Focus must return to Tests after screenshot Enter"
        );

        // Status key should work
        set_status(&mut state, Status::Inconclusive);
        assert_eq!(state.results.results[0].status, Status::Inconclusive);

        // Screenshot was actually saved
        assert_eq!(state.results.results[0].screenshots.len(), 1);
    }
}
