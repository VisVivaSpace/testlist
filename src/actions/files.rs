//! File I/O operations for testlist and results.

use crate::data::definition::Testlist;
use crate::data::results::TestlistResults;
use crate::error::Result;
use std::path::Path;

/// Load a testlist definition from a RON file.
pub fn load_testlist(path: &Path) -> Result<Testlist> {
    Testlist::load(path)
}

/// Load results from a RON file, with backward compatibility migration.
pub fn load_results(path: &Path, testlist: &Testlist) -> Result<TestlistResults> {
    TestlistResults::load(path, testlist)
}

/// Save results to a RON file.
pub fn save_results(results: &TestlistResults, path: &Path) -> Result<()> {
    results.save(path)
}

/// Create a new testlist template file.
pub fn create_template(path: &Path) -> std::io::Result<()> {
    let template = r##"Testlist(
    meta: Meta(
        title: "My Test Checklist",
        description: "Description of what you're testing",
        created: "2025-01-24T00:00:00Z",
        version: "1",
    ),
    tests: [
        Test(
            id: "build",
            title: "Build the project",
            description: "Verify the project builds without errors.",
            setup: [],
            action: "Run the build command",
            verify: [
                "Build completes without errors",
                "No warnings in output",
            ],
            suggested_command: Some("cargo build"),
        ),
        Test(
            id: "tests",
            title: "Run test suite",
            description: "Verify all tests pass.",
            setup: [
                "Ensure build completed successfully",
            ],
            action: "Run the test suite",
            verify: [
                "All tests pass",
                "No flaky tests",
            ],
            suggested_command: Some("cargo test"),
        ),
        Test(
            id: "manual-check",
            title: "Manual verification",
            description: r#"
Perform manual testing of the application.

Pay attention to:
- User interface responsiveness
- Error handling
- Edge cases
            "#,
            setup: [
                "Start the application",
                "Prepare test data",
            ],
            action: "Test the main features manually",
            verify: [
                "Features work as expected",
                "No crashes or errors",
                "Performance is acceptable",
            ],
            suggested_command: None,
        ),
    ],
)
"##;
    std::fs::write(path, template)
}
