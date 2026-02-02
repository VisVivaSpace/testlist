//! Queries related to checklist item states.

use crate::data::results::{checklist_key, ChecklistSection, TestlistResults};

/// Check if a checklist item is checked.
pub fn is_checked(
    results: &TestlistResults,
    test_id: &str,
    section: ChecklistSection,
    item_id: &str,
) -> bool {
    let key = checklist_key(test_id, section, item_id);
    results
        .checklist_results
        .get(&key)
        .copied()
        .unwrap_or(false)
}

/// Get checklist progress for a test section: (checked_count, total_count).
pub fn checklist_progress(
    results: &TestlistResults,
    test_id: &str,
    section: ChecklistSection,
    item_ids: &[&str],
) -> (usize, usize) {
    let checked = item_ids
        .iter()
        .filter(|id| is_checked(results, test_id, section, id))
        .count();
    (checked, item_ids.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::definition::{ChecklistItem, Meta, Test, Testlist};
    use crate::data::results::TestlistResults;

    fn make_results() -> TestlistResults {
        let testlist = Testlist {
            meta: Meta {
                title: "Test".to_string(),
                description: "".to_string(),
                created: "".to_string(),
                version: "1".to_string(),
            },
            tests: vec![Test {
                id: "t1".to_string(),
                title: "Test".to_string(),
                description: "".to_string(),
                setup: vec![
                    ChecklistItem {
                        id: "s0".to_string(),
                        text: "Step".to_string(),
                    },
                    ChecklistItem {
                        id: "s1".to_string(),
                        text: "Step".to_string(),
                    },
                ],
                action: "Act".to_string(),
                verify: vec![ChecklistItem {
                    id: "v0".to_string(),
                    text: "Check".to_string(),
                }],
                suggested_command: None,
            }],
        };
        let mut results = TestlistResults::new_for_testlist(&testlist, "test.ron", "tester");
        results
            .checklist_results
            .insert("t1:setup:s0".to_string(), true);
        results
            .checklist_results
            .insert("t1:setup:s1".to_string(), false);
        results
            .checklist_results
            .insert("t1:verify:v0".to_string(), true);
        results
    }

    #[test]
    fn test_is_checked() {
        let results = make_results();
        assert!(is_checked(&results, "t1", ChecklistSection::Setup, "s0"));
        assert!(!is_checked(&results, "t1", ChecklistSection::Setup, "s1"));
        assert!(is_checked(&results, "t1", ChecklistSection::Verify, "v0"));
        // Non-existent key returns false
        assert!(!is_checked(
            &results,
            "t1",
            ChecklistSection::Verify,
            "v999"
        ));
    }

    #[test]
    fn test_checklist_progress() {
        let results = make_results();
        let (checked, total) =
            checklist_progress(&results, "t1", ChecklistSection::Setup, &["s0", "s1"]);
        assert_eq!(checked, 1);
        assert_eq!(total, 2);

        let (checked, total) =
            checklist_progress(&results, "t1", ChecklistSection::Verify, &["v0"]);
        assert_eq!(checked, 1);
        assert_eq!(total, 1);
    }
}
