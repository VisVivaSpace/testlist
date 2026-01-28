//! RON schema types for testlist files and results.

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Metadata for a testlist definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub title: String,
    pub description: String,
    pub created: String,
    pub version: String,
}

/// A single test item to verify.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Test {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub setup: Vec<String>,
    pub action: String,
    #[serde(default)]
    pub verify: Vec<String>,
    pub suggested_command: Option<String>,
}

/// Root type for testlist definition files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Testlist {
    pub meta: Meta,
    pub tests: Vec<Test>,
}

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

/// Metadata for a results file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultsMeta {
    pub testlist: String,
    pub tester: String,
    pub started: String,
    pub completed: Option<String>,
}

/// Result for a single test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_id: String,
    pub status: Status,
    pub notes: Option<String>,
    #[serde(default)]
    pub screenshots: Vec<PathBuf>,
    pub completed_at: Option<String>,
    pub setup_checked: Option<Vec<bool>>,
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
            setup_checked: if test.setup.is_empty() {
                None
            } else {
                Some(vec![false; test.setup.len()])
            },
            verify_checked: if test.verify.is_empty() {
                None
            } else {
                Some(vec![false; test.verify.len()])
            },
        }
    }
}

/// Root type for results files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Results {
    pub meta: ResultsMeta,
    pub results: Vec<TestResult>,
}

impl Testlist {
    /// Load a testlist from a RON file.
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let testlist: Testlist = ron::from_str(&content)?;
        Ok(testlist)
    }
}

impl Results {
    /// Load results from a RON file.
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let results: Results = ron::from_str(&content)?;
        Ok(results)
    }

    /// Save results to a RON file.
    pub fn save(&self, path: &std::path::Path) -> Result<()> {
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
            results: testlist
                .tests
                .iter()
                .map(TestResult::new_pending)
                .collect(),
        }
    }

    /// Get mutable reference to result for a test by ID.
    pub fn get_result_mut(&mut self, test_id: &str) -> Option<&mut TestResult> {
        self.results.iter_mut().find(|r| r.test_id == test_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_testlist() {
        let ron_str = r#"
Testlist(
    meta: Meta(
        title: "Test Checklist",
        description: "A test checklist",
        created: "2025-01-24T10:00:00Z",
        version: "1",
    ),
    tests: [
        Test(
            id: "build",
            title: "Build successfully",
            description: "Verify the build completes",
            setup: [],
            action: "Run cargo build",
            verify: [
                "Build completes without errors",
            ],
            suggested_command: Some("cargo build"),
        ),
    ],
)
"#;
        let testlist: Testlist = ron::from_str(ron_str).unwrap();
        assert_eq!(testlist.meta.title, "Test Checklist");
        assert_eq!(testlist.tests.len(), 1);
        assert_eq!(testlist.tests[0].id, "build");
        assert_eq!(
            testlist.tests[0].suggested_command,
            Some("cargo build".to_string())
        );
    }

    #[test]
    fn test_parse_results() {
        let ron_str = r#"
Results(
    meta: ResultsMeta(
        testlist: "test.testlist.ron",
        tester: "alice",
        started: "2025-01-24T14:30:00Z",
        completed: None,
    ),
    results: [
        TestResult(
            test_id: "build",
            status: Passed,
            notes: Some("Worked fine"),
            screenshots: [],
            completed_at: Some("2025-01-24T14:32:00Z"),
            setup_checked: None,
            verify_checked: Some([true]),
        ),
    ],
)
"#;
        let results: Results = ron::from_str(ron_str).unwrap();
        assert_eq!(results.meta.tester, "alice");
        assert_eq!(results.results.len(), 1);
        assert_eq!(results.results[0].status, Status::Passed);
    }

    #[test]
    fn test_status_default() {
        assert_eq!(Status::default(), Status::Pending);
    }

    #[test]
    fn test_new_pending_result() {
        let test = Test {
            id: "test1".to_string(),
            title: "Test".to_string(),
            description: "Desc".to_string(),
            setup: vec!["Step 1".to_string(), "Step 2".to_string()],
            action: "Do it".to_string(),
            verify: vec!["Check 1".to_string()],
            suggested_command: None,
        };
        let result = TestResult::new_pending(&test);
        assert_eq!(result.test_id, "test1");
        assert_eq!(result.status, Status::Pending);
        assert_eq!(result.setup_checked, Some(vec![false, false]));
        assert_eq!(result.verify_checked, Some(vec![false]));
    }

    #[test]
    fn test_new_pending_result_no_setup_verify() {
        let test = Test {
            id: "simple".to_string(),
            title: "Simple Test".to_string(),
            description: "A test with no setup or verify".to_string(),
            setup: vec![],
            action: "Just do it".to_string(),
            verify: vec![],
            suggested_command: Some("echo test".to_string()),
        };
        let result = TestResult::new_pending(&test);
        assert_eq!(result.setup_checked, None);
        assert_eq!(result.verify_checked, None);
        assert!(result.notes.is_none());
        assert!(result.screenshots.is_empty());
    }

    #[test]
    fn test_results_new_for_testlist() {
        let testlist = Testlist {
            meta: Meta {
                title: "Test".to_string(),
                description: "Desc".to_string(),
                created: "2025-01-24".to_string(),
                version: "1".to_string(),
            },
            tests: vec![
                Test {
                    id: "t1".to_string(),
                    title: "Test 1".to_string(),
                    description: "".to_string(),
                    setup: vec![],
                    action: "Act".to_string(),
                    verify: vec![],
                    suggested_command: None,
                },
                Test {
                    id: "t2".to_string(),
                    title: "Test 2".to_string(),
                    description: "".to_string(),
                    setup: vec!["Step".to_string()],
                    action: "Act".to_string(),
                    verify: vec![],
                    suggested_command: None,
                },
            ],
        };
        let results = Results::new_for_testlist(&testlist, "test.ron", "tester");
        assert_eq!(results.meta.tester, "tester");
        assert_eq!(results.meta.testlist, "test.ron");
        assert_eq!(results.results.len(), 2);
        assert_eq!(results.results[0].test_id, "t1");
        assert_eq!(results.results[1].test_id, "t2");
        assert!(results.results[0].setup_checked.is_none());
        assert_eq!(results.results[1].setup_checked, Some(vec![false]));
    }

    #[test]
    fn test_results_get_result_mut() {
        let testlist = Testlist {
            meta: Meta {
                title: "Test".to_string(),
                description: "".to_string(),
                created: "".to_string(),
                version: "1".to_string(),
            },
            tests: vec![
                Test {
                    id: "existing".to_string(),
                    title: "Test".to_string(),
                    description: "".to_string(),
                    setup: vec![],
                    action: "Act".to_string(),
                    verify: vec![],
                    suggested_command: None,
                },
            ],
        };
        let mut results = Results::new_for_testlist(&testlist, "test.ron", "tester");

        // Test finding existing result
        let result = results.get_result_mut("existing");
        assert!(result.is_some());
        result.unwrap().status = Status::Passed;

        // Test finding non-existing result
        assert!(results.get_result_mut("nonexistent").is_none());

        // Verify the mutation worked
        assert_eq!(results.results[0].status, Status::Passed);
    }

    #[test]
    fn test_parse_all_statuses() {
        let ron_str = r#"
Results(
    meta: ResultsMeta(
        testlist: "test.ron",
        tester: "user",
        started: "2025-01-24",
        completed: None,
    ),
    results: [
        TestResult(test_id: "t1", status: Pending, notes: None, screenshots: [], completed_at: None, setup_checked: None, verify_checked: None),
        TestResult(test_id: "t2", status: Passed, notes: None, screenshots: [], completed_at: None, setup_checked: None, verify_checked: None),
        TestResult(test_id: "t3", status: Failed, notes: None, screenshots: [], completed_at: None, setup_checked: None, verify_checked: None),
        TestResult(test_id: "t4", status: Inconclusive, notes: None, screenshots: [], completed_at: None, setup_checked: None, verify_checked: None),
        TestResult(test_id: "t5", status: Skipped, notes: None, screenshots: [], completed_at: None, setup_checked: None, verify_checked: None),
    ],
)
"#;
        let results: Results = ron::from_str(ron_str).unwrap();
        assert_eq!(results.results[0].status, Status::Pending);
        assert_eq!(results.results[1].status, Status::Passed);
        assert_eq!(results.results[2].status, Status::Failed);
        assert_eq!(results.results[3].status, Status::Inconclusive);
        assert_eq!(results.results[4].status, Status::Skipped);
    }

    #[test]
    fn test_parse_with_screenshots() {
        let ron_str = r#"
Results(
    meta: ResultsMeta(
        testlist: "test.ron",
        tester: "user",
        started: "2025-01-24",
        completed: None,
    ),
    results: [
        TestResult(
            test_id: "t1",
            status: Failed,
            notes: Some("Found a bug"),
            screenshots: ["/tmp/bug1.png", "/tmp/bug2.png"],
            completed_at: Some("2025-01-24T15:00:00Z"),
            setup_checked: None,
            verify_checked: Some([true, false]),
        ),
    ],
)
"#;
        let results: Results = ron::from_str(ron_str).unwrap();
        assert_eq!(results.results[0].screenshots.len(), 2);
        assert_eq!(results.results[0].notes, Some("Found a bug".to_string()));
        assert_eq!(results.results[0].verify_checked, Some(vec![true, false]));
    }

    #[test]
    fn test_results_save_load_roundtrip() {
        let testlist = Testlist {
            meta: Meta {
                title: "Roundtrip Test".to_string(),
                description: "Testing save/load".to_string(),
                created: "2025-01-24".to_string(),
                version: "1".to_string(),
            },
            tests: vec![
                Test {
                    id: "test1".to_string(),
                    title: "Test".to_string(),
                    description: "".to_string(),
                    setup: vec!["Setup step".to_string()],
                    action: "Act".to_string(),
                    verify: vec!["Verify step".to_string()],
                    suggested_command: Some("echo hello".to_string()),
                },
            ],
        };

        let mut results = Results::new_for_testlist(&testlist, "test.ron", "alice");
        results.results[0].status = Status::Passed;
        results.results[0].notes = Some("This worked!".to_string());
        results.results[0].setup_checked = Some(vec![true]);
        results.results[0].verify_checked = Some(vec![true]);

        // Create a temp file
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // Save results
        results.save(&temp_path).unwrap();

        // Load results
        let loaded = Results::load(&temp_path).unwrap();

        // Verify roundtrip
        assert_eq!(loaded.meta.tester, "alice");
        assert_eq!(loaded.results[0].status, Status::Passed);
        assert_eq!(loaded.results[0].notes, Some("This worked!".to_string()));
        assert_eq!(loaded.results[0].setup_checked, Some(vec![true]));
    }
}
