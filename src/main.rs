mod error;
mod schema;
mod tui;

use clap::Parser;
use std::path::PathBuf;

/// Structured human feedback collection tool
#[derive(Parser, Debug)]
#[command(name = "testlist")]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to testlist definition file
    #[arg(value_name = "TESTLIST")]
    testlist: Option<PathBuf>,

    /// Create a new testlist template at the specified path
    #[arg(long, value_name = "PATH")]
    new: Option<PathBuf>,

    /// Set tester name for results (default: $USER)
    #[arg(long, value_name = "NAME")]
    tester: Option<String>,

    /// Custom path for results file (default: <testlist>.results.ron)
    #[arg(long, value_name = "PATH")]
    results: Option<PathBuf>,

    /// Continue from existing results file
    #[arg(long, name = "continue")]
    continue_from: bool,
}

fn main() {
    let args = Args::parse();

    // Handle --new flag: create template and exit
    if let Some(path) = args.new {
        if let Err(e) = create_template(&path) {
            eprintln!("Error creating template: {}", e);
            std::process::exit(1);
        }
        println!("Created testlist template at: {}", path.display());
        return;
    }

    // Require testlist file for normal operation
    let Some(testlist_path) = args.testlist else {
        eprintln!("Error: Must provide a testlist file or use --new to create one");
        eprintln!("Usage: testlist <TESTLIST> or testlist --new <PATH>");
        std::process::exit(1);
    };

    // Get tester name
    let tester = args.tester.unwrap_or_else(|| {
        std::env::var("USER").unwrap_or_else(|_| "unknown".to_string())
    });

    // Determine results path
    let results_path = args.results.unwrap_or_else(|| {
        let mut path = testlist_path.clone();
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let new_name = if stem.ends_with(".testlist") {
            format!("{}.results.ron", stem)
        } else {
            format!("{}.results.ron", stem)
        };
        path.set_file_name(new_name);
        path
    });

    // Load testlist
    let testlist = match schema::Testlist::load(&testlist_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error loading testlist: {}", e);
            std::process::exit(1);
        }
    };

    // Load or create results
    let results = if args.continue_from && results_path.exists() {
        match schema::Results::load(&results_path) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error loading results: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        schema::Results::new_for_testlist(
            &testlist,
            &testlist_path.to_string_lossy(),
            &tester,
        )
    };

    // Create app state and run TUI
    let mut state = tui::AppState::new(testlist, results, testlist_path, results_path.clone());

    if let Err(e) = tui::run(&mut state) {
        eprintln!("Error running TUI: {}", e);
        std::process::exit(1);
    }

    // Save results on exit
    if let Err(e) = state.results.save(&results_path) {
        eprintln!("Error saving results: {}", e);
        std::process::exit(1);
    }

    println!("Results saved to: {}", results_path.display());
}

/// Create a new testlist template file.
fn create_template(path: &PathBuf) -> std::io::Result<()> {
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
