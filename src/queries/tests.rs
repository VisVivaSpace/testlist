//! Queries related to tests and results.

use crate::data::definition::Test;
use crate::data::results::{Status, TestResult, TestlistResults};
use crate::data::state::AppState;

/// Get the currently selected test definition.
pub fn current_test(state: &AppState) -> Option<&Test> {
    state.testlist.tests.get(state.selected_test)
}

/// Get the result for the currently selected test.
pub fn current_result(state: &AppState) -> Option<&TestResult> {
    current_test(state).and_then(|t| state.results.results.iter().find(|r| r.test_id == t.id))
}

/// Get the result for a specific test by ID.
pub fn result_for_test<'a>(results: &'a TestlistResults, test_id: &str) -> Option<&'a TestResult> {
    results.results.iter().find(|r| r.test_id == test_id)
}

/// Count completed (non-pending) tests.
pub fn completed_count(state: &AppState) -> usize {
    state
        .results
        .results
        .iter()
        .filter(|r| r.status != Status::Pending)
        .count()
}

/// Calculate the line number of the current selection (header) in the tests pane.
pub fn selected_line_number(state: &AppState) -> usize {
    let mut line = 0;

    for (i, test) in state.testlist.tests.iter().enumerate() {
        if i == state.selected_test {
            return line;
        }
        line += 1;

        if state.expanded_tests.contains(&test.id) {
            if !test.setup.is_empty() {
                line += 1 + test.setup.len(); // "Setup:" + items
            }
            line += 1; // Action
            if !test.verify.is_empty() {
                line += 1 + test.verify.len(); // "Verify:" + items
            }
        }
    }

    line
}

/// Map a y-coordinate in the tests pane to a test index.
/// Clicks on expanded content rows map to the parent test.
pub fn map_y_to_test_index(state: &AppState, y: usize) -> Option<usize> {
    let mut current_y = 0;

    for (i, test) in state.testlist.tests.iter().enumerate() {
        let header_y = current_y;
        current_y += 1;

        if state.expanded_tests.contains(&test.id) {
            if !test.setup.is_empty() {
                current_y += 1 + test.setup.len();
            }
            current_y += 1; // Action
            if !test.verify.is_empty() {
                current_y += 1 + test.verify.len();
            }
        }

        // y falls within this test's range (header + expanded content)
        if y >= header_y && y < current_y {
            return Some(i);
        }
    }

    None
}

#[cfg(test)]
mod tests_mod {
    use super::*;
    use crate::data::definition::{ChecklistItem, Meta, Testlist};
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
                        text: "Step A".to_string(),
                    }],
                    action: "Do it".to_string(),
                    verify: vec![],
                    suggested_command: None,
                },
                Test {
                    id: "t2".to_string(),
                    title: "Test 2".to_string(),
                    description: "".to_string(),
                    setup: vec![],
                    action: "Do it".to_string(),
                    verify: vec![ChecklistItem {
                        id: "v0".to_string(),
                        text: "Check".to_string(),
                    }],
                    suggested_command: Some("echo hi".to_string()),
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
    fn test_current_test() {
        let state = make_state();
        let test = current_test(&state).unwrap();
        assert_eq!(test.id, "t1");
    }

    #[test]
    fn test_current_result() {
        let state = make_state();
        let result = current_result(&state).unwrap();
        assert_eq!(result.test_id, "t1");
        assert_eq!(result.status, Status::Pending);
    }

    #[test]
    fn test_completed_count() {
        let mut state = make_state();
        assert_eq!(completed_count(&state), 0);
        state.results.results[0].status = Status::Passed;
        assert_eq!(completed_count(&state), 1);
    }

    #[test]
    fn test_map_y_expanded_content_maps_to_parent() {
        let mut state = make_state();
        state.expanded_tests.insert("t1".to_string());
        // t1 layout: header(0), "Setup:"(1), "Step A"(2), Action(3)
        // t2 starts at y=4
        assert_eq!(map_y_to_test_index(&state, 0), Some(0)); // header
        assert_eq!(map_y_to_test_index(&state, 1), Some(0)); // Setup:
        assert_eq!(map_y_to_test_index(&state, 2), Some(0)); // Step A
        assert_eq!(map_y_to_test_index(&state, 3), Some(0)); // Action
        assert_eq!(map_y_to_test_index(&state, 4), Some(1)); // t2 header
    }
}
