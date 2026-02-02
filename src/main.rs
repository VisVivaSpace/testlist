use clap::Parser;
use std::path::PathBuf;

use testlist::actions::files;
use testlist::data::results::TestlistResults;
use testlist::data::state::AppState;

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
        if let Err(e) = files::create_template(&path) {
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
    let tester = args
        .tester
        .unwrap_or_else(|| std::env::var("USER").unwrap_or_else(|_| "unknown".to_string()));

    // Determine results path
    let results_path = args.results.unwrap_or_else(|| {
        let mut path = testlist_path.clone();
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let new_name = format!("{}.results.ron", stem);
        path.set_file_name(new_name);
        path
    });

    // Load testlist
    let testlist = match files::load_testlist(&testlist_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error loading testlist: {}", e);
            std::process::exit(1);
        }
    };

    // Load or create results
    let results = if args.continue_from && results_path.exists() {
        match files::load_results(&results_path, &testlist) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error loading results: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        TestlistResults::new_for_testlist(&testlist, &testlist_path.to_string_lossy(), &tester)
    };

    // Create app state and run TUI
    let mut state = AppState::new(testlist, results, testlist_path, results_path.clone());

    if let Err(e) = testlist::ui::app::run(&mut state) {
        eprintln!("Error running TUI: {}", e);
        std::process::exit(1);
    }

    // Save results on exit (unless user chose to quit without saving)
    if !state.skip_save {
        if let Err(e) = files::save_results(&state.results, &results_path) {
            eprintln!("Error saving results: {}", e);
            std::process::exit(1);
        }
        println!("Results saved to: {}", results_path.display());
    }
}
