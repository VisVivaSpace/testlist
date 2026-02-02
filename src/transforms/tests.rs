//! Transforms for test status and checklist toggling.

use crate::data::results::{checklist_key, ChecklistSection, Status};
use crate::data::state::{AppState, SubSelection};
use crate::queries::tests::current_test;

/// Set the status of the currently selected test.
pub fn set_status(state: &mut AppState, status: Status) {
    let test_id = match current_test(state) {
        Some(t) => t.id.clone(),
        None => return,
    };
    if let Some(result) = state.results.get_result_mut(&test_id) {
        result.status = status;
        result.completed_at = Some(chrono::Utc::now().to_rfc3339());
        state.dirty = true;
    }
}

/// Toggle a checklist item (setup or verify) for the currently selected test.
pub fn toggle_checklist(state: &mut AppState) {
    let test = match current_test(state) {
        Some(t) => t,
        None => return,
    };

    let key = match state.sub_selection {
        SubSelection::Setup(i) => test
            .setup
            .get(i)
            .map(|item| checklist_key(&test.id, ChecklistSection::Setup, &item.id)),
        SubSelection::Verify(i) => test
            .verify
            .get(i)
            .map(|item| checklist_key(&test.id, ChecklistSection::Verify, &item.id)),
        _ => None,
    };

    if let Some(key) = key {
        let current = state
            .results
            .checklist_results
            .get(&key)
            .copied()
            .unwrap_or(false);
        state.results.checklist_results.insert(key, !current);
        state.dirty = true;
    }
}

#[cfg(test)]
mod tests_mod {
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
    fn test_set_status() {
        let mut state = make_state();
        set_status(&mut state, Status::Passed);
        assert_eq!(state.results.results[0].status, Status::Passed);
        assert!(state.results.results[0].completed_at.is_some());
        assert!(state.dirty);
    }

    #[test]
    fn test_toggle_checklist_setup() {
        let mut state = make_state();
        state.sub_selection = SubSelection::Setup(0);
        // Expand the test so sub_selection is valid
        state.expanded_tests.insert("t1".to_string());

        toggle_checklist(&mut state);
        assert_eq!(
            state.results.checklist_results.get("t1:setup:s0"),
            Some(&true)
        );
        assert!(state.dirty);

        // Toggle again
        state.dirty = false;
        toggle_checklist(&mut state);
        assert_eq!(
            state.results.checklist_results.get("t1:setup:s0"),
            Some(&false)
        );
        assert!(state.dirty);
    }

    #[test]
    fn test_toggle_checklist_verify() {
        let mut state = make_state();
        state.sub_selection = SubSelection::Verify(0);
        toggle_checklist(&mut state);
        assert_eq!(
            state.results.checklist_results.get("t1:verify:v0"),
            Some(&true)
        );
    }

    #[test]
    fn test_toggle_checklist_header_noop() {
        let mut state = make_state();
        state.sub_selection = SubSelection::Header;
        toggle_checklist(&mut state);
        assert!(!state.dirty);
    }
}
