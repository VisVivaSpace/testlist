//! Transforms for navigation within the tests pane.

use crate::data::state::{AppState, SubSelection};
use crate::queries::tests::{current_test, selected_line_number};

/// Navigate down in the tests pane.
pub fn select_next(state: &mut AppState) {
    let Some(test) = current_test(state) else {
        return;
    };
    let is_expanded = state.expanded_tests.contains(&test.id);

    if !is_expanded {
        if state.selected_test < state.testlist.tests.len().saturating_sub(1) {
            state.selected_test += 1;
            state.sub_selection = SubSelection::Header;
        }
        return;
    }

    let setup_count = test.setup.len();
    let verify_count = test.verify.len();

    match state.sub_selection {
        SubSelection::Header => {
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
            } else if state.selected_test < state.testlist.tests.len().saturating_sub(1) {
                state.selected_test += 1;
                state.sub_selection = SubSelection::Header;
            }
        }
        SubSelection::Verify(i) => {
            if i + 1 < verify_count {
                state.sub_selection = SubSelection::Verify(i + 1);
            } else if state.selected_test < state.testlist.tests.len().saturating_sub(1) {
                state.selected_test += 1;
                state.sub_selection = SubSelection::Header;
            }
        }
    }
}

/// Navigate up in the tests pane.
pub fn select_prev(state: &mut AppState) {
    let Some(test) = current_test(state) else {
        return;
    };
    let is_expanded = state.expanded_tests.contains(&test.id);

    if state.sub_selection == SubSelection::Header {
        if state.selected_test > 0 {
            state.selected_test -= 1;
            if let Some(prev_test) = current_test(state) {
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
        assert_eq!(state.sub_selection, SubSelection::Header);
    }

    #[test]
    fn test_select_next_expanded() {
        let mut state = make_state();
        state.expanded_tests.insert("t1".to_string());
        // Header -> Setup(0)
        select_next(&mut state);
        assert_eq!(state.sub_selection, SubSelection::Setup(0));
        // Setup(0) -> Action
        select_next(&mut state);
        assert_eq!(state.sub_selection, SubSelection::Action);
        // Action -> Verify(0)
        select_next(&mut state);
        assert_eq!(state.sub_selection, SubSelection::Verify(0));
        // Verify(0) -> next test
        select_next(&mut state);
        assert_eq!(state.selected_test, 1);
        assert_eq!(state.sub_selection, SubSelection::Header);
    }

    #[test]
    fn test_select_prev_to_expanded() {
        let mut state = make_state();
        state.expanded_tests.insert("t1".to_string());
        state.selected_test = 1;
        state.sub_selection = SubSelection::Header;
        // Should go to last item of expanded t1
        select_prev(&mut state);
        assert_eq!(state.selected_test, 0);
        assert_eq!(state.sub_selection, SubSelection::Verify(0));
    }

    #[test]
    fn test_select_prev_at_top() {
        let mut state = make_state();
        select_prev(&mut state);
        assert_eq!(state.selected_test, 0);
        assert_eq!(state.sub_selection, SubSelection::Header);
    }
}
