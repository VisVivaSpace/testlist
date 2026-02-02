//! Transforms for navigation within the tests pane.

use crate::data::state::AppState;
use crate::queries::tests::selected_line_number;

/// Navigate down in the tests pane — always moves between test headers.
pub fn select_next(state: &mut AppState) {
    if state.selected_test < state.testlist.tests.len().saturating_sub(1) {
        state.selected_test += 1;
    }
}

/// Navigate up in the tests pane — always moves between test headers.
pub fn select_prev(state: &mut AppState) {
    if state.selected_test > 0 {
        state.selected_test -= 1;
    }
}

/// Adjust scroll offset to keep selection visible.
pub fn adjust_scroll(state: &mut AppState) {
    let selected = selected_line_number(state);
    let visible = state.tests_visible_height;

    if selected < state.tests_scroll_offset {
        state.tests_scroll_offset = selected;
    } else if selected >= state.tests_scroll_offset + visible {
        state.tests_scroll_offset = selected.saturating_sub(visible) + 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::definition::{ChecklistItem, Meta, Test, Testlist};
    use crate::data::results::TestlistResults;

    fn make_state() -> AppState {
        let testlist = Testlist {
            meta: Meta {
                title: "Test".to_string(),
                description: "".to_string(),
                created: "".to_string(),
                version: "1".to_string(),
            },
            tests: vec![
                Test {
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
                },
                Test {
                    id: "t2".to_string(),
                    title: "Test 2".to_string(),
                    description: "".to_string(),
                    setup: vec![],
                    action: "Do it".to_string(),
                    verify: vec![],
                    suggested_command: None,
                },
            ],
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
    fn test_select_next_collapsed() {
        let mut state = make_state();
        assert_eq!(state.selected_test, 0);
        select_next(&mut state);
        assert_eq!(state.selected_test, 1);
    }

    #[test]
    fn test_select_next_expanded_skips_content() {
        let mut state = make_state();
        state.expanded_tests.insert("t1".to_string());
        // Should jump directly to next test header
        select_next(&mut state);
        assert_eq!(state.selected_test, 1);
    }

    #[test]
    fn test_select_prev_skips_expanded() {
        let mut state = make_state();
        state.expanded_tests.insert("t1".to_string());
        state.selected_test = 1;
        // Should jump directly to previous test header
        select_prev(&mut state);
        assert_eq!(state.selected_test, 0);
    }

    #[test]
    fn test_select_prev_at_top() {
        let mut state = make_state();
        select_prev(&mut state);
        assert_eq!(state.selected_test, 0);
    }
}
