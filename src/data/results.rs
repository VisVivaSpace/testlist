//! Types for testlist results files (.testlist.results.ron).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::definition::{Test, Testlist};

/// Status of a test result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Status {
    #[default]
    Pending,
    Passed,
    Failed,
    Inconclusive,
    Skipped,
}

/// Checklist section type for composite keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChecklistSection {
    Setup,
    Verify,
}

impl std::fmt::Display for ChecklistSection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChecklistSection::Setup => write!(f, "setup"),
            ChecklistSection::Verify => write!(f, "verify"),
        }
    }
}

/// Metadata for a results file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultsMeta {
    pub testlist: String,
    pub tester: String,
    pub started: String,
    pub completed: Option<String>,
}

/// Result for a single test.
///
/// Checklist state is stored in the parent `TestlistResults.checklist_results`
/// using composite keys like `"test-id:setup:item-id"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_id: String,
    pub status: Status,
    pub notes: Option<String>,
    #[serde(default)]
    pub screenshots: Vec<PathBuf>,
    pub completed_at: Option<String>,
    // Legacy fields for backward compatibility on load.
    // Always None when saving in new format.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setup_checked: Option<Vec<bool>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verify_checked: Option<Vec<bool>>,
}

impl TestResult {
    /// Create a new pending result for a test.
    pub fn new_pending(test: &Test) -> Self {
        Self {
            test_id: test.id.clone(),
            status: Status::Pending,
            notes: None,
            screenshots: Vec::new(),
            completed_at: None,
            setup_checked: None,
            verify_checked: None,
        }
    }
}

/// Builds a composite key for the checklist_results HashMap.
pub fn checklist_key(test_id: &str, section: ChecklistSection, item_id: &str) -> String {
    format!("{}:{}:{}", test_id, section, item_id)
}

/// Root type for results files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestlistResults {
    pub meta: ResultsMeta,
    pub results: Vec<TestResult>,
    /// Checklist item states with composite keys: "test-id:setup:item-id" or "test-id:verify:item-id"
    #[serde(default)]
    pub checklist_results: HashMap<String, bool>,
}

impl TestlistResults {
    /// Load results from a RON file, migrating old format if needed.
    pub fn load(path: &std::path::Path, testlist: &Testlist) -> crate::error::Result<Self> {
        let content = std::fs::read_to_string(path)?;

        // Try loading as new format first
        if let Ok(results) = ron::from_str::<TestlistResults>(&content) {
            return Ok(results);
        }

        // Fall back to old format and migrate
        let old: OldResults = ron::from_str(&content)?;
        Ok(Self::migrate_from_old(old, testlist))
    }

    /// Save results to a RON file.
    pub fn save(&self, path: &std::path::Path) -> crate::error::Result<()> {
        let content = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Create initial results for a testlist.
    pub fn new_for_testlist(testlist: &Testlist, testlist_path: &str, tester: &str) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            meta: ResultsMeta {
                testlist: testlist_path.to_string(),
                tester: tester.to_string(),
                started: now,
                completed: None,
            },
            results: testlist.tests.iter().map(TestResult::new_pending).collect(),
            checklist_results: HashMap::new(),
        }
    }

    /// Get mutable reference to result for a test by ID.
    pub fn get_result_mut(&mut self, test_id: &str) -> Option<&mut TestResult> {
        self.results.iter_mut().find(|r| r.test_id == test_id)
    }

    /// Migrate from old Results format (with setup_checked/verify_checked on each TestResult)
    /// to new format with centralized checklist_results HashMap.
    fn migrate_from_old(old: OldResults, testlist: &Testlist) -> Self {
        let mut checklist_results = HashMap::new();

        for old_result in &old.results {
            // Find the corresponding test definition to get item IDs
            if let Some(test) = testlist.tests.iter().find(|t| t.id == old_result.test_id) {
                // Migrate setup_checked
                if let Some(ref checked) = old_result.setup_checked {
                    for (i, &val) in checked.iter().enumerate() {
                        if let Some(item) = test.setup.get(i) {
                            let key = checklist_key(
                                &old_result.test_id,
                                ChecklistSection::Setup,
                                &item.id,
                            );
                            checklist_results.insert(key, val);
                        }
                    }
                }
                // Migrate verify_checked
                if let Some(ref checked) = old_result.verify_checked {
                    for (i, &val) in checked.iter().enumerate() {
                        if let Some(item) = test.verify.get(i) {
                            let key = checklist_key(
                                &old_result.test_id,
                                ChecklistSection::Verify,
                                &item.id,
                            );
                            checklist_results.insert(key, val);
                        }
                    }
                }
            }
        }

        // Convert results, clearing legacy fields
        let results = old
            .results
            .into_iter()
            .map(|r| TestResult {
                test_id: r.test_id,
                status: r.status,
                notes: r.notes,
                screenshots: r.screenshots,
                completed_at: r.completed_at,
                setup_checked: None,
                verify_checked: None,
            })
            .collect();

        TestlistResults {
            meta: old.meta,
            results,
            checklist_results,
        }
    }
}

/// Old format for backward compatibility loading.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Results")]
struct OldResults {
    meta: ResultsMeta,
    results: Vec<OldTestResult>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "TestResult")]
struct OldTestResult {
    test_id: String,
    status: Status,
    notes: Option<String>,
    #[serde(default)]
    screenshots: Vec<PathBuf>,
    completed_at: Option<String>,
    setup_checked: Option<Vec<bool>>,
    verify_checked: Option<Vec<bool>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::definition::{ChecklistItem, Meta};

    fn make_testlist() -> Testlist {
        Testlist {
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
                setup: vec![
                    ChecklistItem {
                        id: "setup-0".to_string(),
                        text: "Step A".to_string(),
                    },
                    ChecklistItem {
                        id: "setup-1".to_string(),
                        text: "Step B".to_string(),
                    },
                ],
                action: "Do it".to_string(),
                verify: vec![ChecklistItem {
                    id: "verify-0".to_string(),
                    text: "Check A".to_string(),
                }],
                suggested_command: None,
            }],
        }
    }

    #[test]
    fn test_status_default() {
        assert_eq!(Status::default(), Status::Pending);
    }

    #[test]
    fn test_new_pending_result() {
        let testlist = make_testlist();
        let result = TestResult::new_pending(&testlist.tests[0]);
        assert_eq!(result.test_id, "t1");
        assert_eq!(result.status, Status::Pending);
        assert!(result.setup_checked.is_none());
        assert!(result.verify_checked.is_none());
    }

    #[test]
    fn test_new_for_testlist() {
        let testlist = make_testlist();
        let results = TestlistResults::new_for_testlist(&testlist, "test.ron", "alice");
        assert_eq!(results.meta.tester, "alice");
        assert_eq!(results.results.len(), 1);
        assert!(results.checklist_results.is_empty());
    }

    #[test]
    fn test_get_result_mut() {
        let testlist = make_testlist();
        let mut results = TestlistResults::new_for_testlist(&testlist, "test.ron", "tester");
        assert!(results.get_result_mut("t1").is_some());
        assert!(results.get_result_mut("nonexistent").is_none());
        results.get_result_mut("t1").unwrap().status = Status::Passed;
        assert_eq!(results.results[0].status, Status::Passed);
    }

    #[test]
    fn test_checklist_key() {
        assert_eq!(
            checklist_key("build", ChecklistSection::Setup, "setup-0"),
            "build:setup:setup-0"
        );
        assert_eq!(
            checklist_key("login", ChecklistSection::Verify, "v1"),
            "login:verify:v1"
        );
    }

    #[test]
    fn test_parse_old_format_results() {
        let ron_str = r#"
Results(
    meta: ResultsMeta(
        testlist: "test.ron",
        tester: "alice",
        started: "2025-01-24T14:30:00Z",
        completed: None,
    ),
    results: [
        TestResult(
            test_id: "t1",
            status: Passed,
            notes: Some("Worked fine"),
            screenshots: [],
            completed_at: Some("2025-01-24T14:32:00Z"),
            setup_checked: Some([true, false]),
            verify_checked: Some([true]),
        ),
    ],
)
"#;
        let old: OldResults = ron::from_str(ron_str).unwrap();
        let testlist = make_testlist();
        let migrated = TestlistResults::migrate_from_old(old, &testlist);

        assert_eq!(migrated.meta.tester, "alice");
        assert_eq!(migrated.results[0].status, Status::Passed);
        assert!(migrated.results[0].setup_checked.is_none()); // Cleared
        assert_eq!(
            migrated.checklist_results.get("t1:setup:setup-0"),
            Some(&true)
        );
        assert_eq!(
            migrated.checklist_results.get("t1:setup:setup-1"),
            Some(&false)
        );
        assert_eq!(
            migrated.checklist_results.get("t1:verify:verify-0"),
            Some(&true)
        );
    }

    #[test]
    fn test_parse_new_format_results() {
        let ron_str = r#"
TestlistResults(
    meta: ResultsMeta(
        testlist: "test.ron",
        tester: "bob",
        started: "2025-01-24",
        completed: None,
    ),
    results: [
        TestResult(
            test_id: "t1",
            status: Passed,
            notes: None,
            screenshots: [],
            completed_at: None,
        ),
    ],
    checklist_results: {
        "t1:setup:setup-0": true,
        "t1:verify:verify-0": false,
    },
)
"#;
        let results: TestlistResults = ron::from_str(ron_str).unwrap();
        assert_eq!(results.meta.tester, "bob");
        assert_eq!(
            results.checklist_results.get("t1:setup:setup-0"),
            Some(&true)
        );
        assert_eq!(
            results.checklist_results.get("t1:verify:verify-0"),
            Some(&false)
        );
    }

    #[test]
    fn test_parse_all_statuses() {
        let ron_str = r#"
TestlistResults(
    meta: ResultsMeta(
        testlist: "test.ron",
        tester: "user",
        started: "2025-01-24",
        completed: None,
    ),
    results: [
        TestResult(test_id: "t1", status: Pending, notes: None, screenshots: [], completed_at: None),
        TestResult(test_id: "t2", status: Passed, notes: None, screenshots: [], completed_at: None),
        TestResult(test_id: "t3", status: Failed, notes: None, screenshots: [], completed_at: None),
        TestResult(test_id: "t4", status: Inconclusive, notes: None, screenshots: [], completed_at: None),
        TestResult(test_id: "t5", status: Skipped, notes: None, screenshots: [], completed_at: None),
    ],
    checklist_results: {},
)
"#;
        let results: TestlistResults = ron::from_str(ron_str).unwrap();
        assert_eq!(results.results[0].status, Status::Pending);
        assert_eq!(results.results[1].status, Status::Passed);
        assert_eq!(results.results[2].status, Status::Failed);
        assert_eq!(results.results[3].status, Status::Inconclusive);
        assert_eq!(results.results[4].status, Status::Skipped);
    }

    #[test]
    fn test_results_save_load_roundtrip() {
        let testlist = make_testlist();
        let mut results = TestlistResults::new_for_testlist(&testlist, "test.ron", "alice");
        results.results[0].status = Status::Passed;
        results.results[0].notes = Some("This worked!".to_string());
        results
            .checklist_results
            .insert("t1:setup:setup-0".to_string(), true);
        results
            .checklist_results
            .insert("t1:verify:verify-0".to_string(), true);

        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        results.save(&temp_path).unwrap();
        let loaded = TestlistResults::load(&temp_path, &testlist).unwrap();

        assert_eq!(loaded.meta.tester, "alice");
        assert_eq!(loaded.results[0].status, Status::Passed);
        assert_eq!(loaded.results[0].notes, Some("This worked!".to_string()));
        assert_eq!(
            loaded.checklist_results.get("t1:setup:setup-0"),
            Some(&true)
        );
        assert_eq!(
            loaded.checklist_results.get("t1:verify:verify-0"),
            Some(&true)
        );
    }
}
