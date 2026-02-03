# testlist

**A TUI tool for structured human feedback collection**

testlist is a terminal-based tool that guides users through manual testing, evaluation, and feedback tasks. It displays a checklist of items to verify, provides an embedded terminal for running commands, and captures structured notes and screenshots.

While automated tests verify *what code does*, testlist captures *what humans observe* — subjective quality, usability, edge cases, and anything that requires human judgment.

```
┌─ Tests ──────────────────────────────────┬─ Notes ─────────────────────────┐
│ [✓] Build successfully                   │ Login button takes ~3 seconds   │
│ [✗] Login flow                           │ to respond. Error message on    │
│ [?] Overall UX impression                │ wrong password is vague.        │
│ [ ] Error handling                       │                                 │
│                                          │ Screenshots:                    │
│ ▼ [✗] Login flow                         │   [1] login-slow.png            │
│   ┌ Setup ────────────────────────────┐  │   [2] error-message.png         │
│   │ • Start dev server                │  │                                 │
│   │ • Open browser                    │  │                                 │
│   │ • Test user exists                │  │                                 │
│   └───────────────────────────────────┘  │                                 │
│   Action: Attempt to log in with the     │                                 │
│           test credentials               │                                 │
│   ┌ Verify ───────────────────────────┐  │                                 │
│   │ • Loading state shown             │  │                                 │
│   │ • Redirected                      │  │                                 │
│   │ • Welcome message                 │  │                                 │
│   │ • Session persists                │  │                                 │
│   └───────────────────────────────────┘  │                                 │
├─ Terminal ───────────────────────────────┴─────────────────────────────────┤
│ $ cargo run --bin server                                                   │
│ Server running on http://localhost:3000                                    │
│ $ _                                                                        │
├────────────────────────────────────────────────────────────────────────────┤
│ [P]ass [F]ail [I]nc [S]kip │ [Tab] Pane │ [?] Help │ [w] Save │ [Q]uit │
└────────────────────────────────────────────────────────────────────────────┘
```

## Installation

### Build from source

```bash
git clone https://github.com/visviva/testlist.git
cd testlist
cargo build --release
# Binary at target/release/testlist
```

### Cargo install

```bash
cargo install --path .
```

## Quick Start

```bash
# Create a new testlist template
testlist --new ./my-tests.testlist.ron

# Edit the generated file to add your tests, then run it
testlist ./my-tests.testlist.ron

# Continue a previous session
testlist ./my-tests.testlist.ron --continue
```

## CLI Usage

```
testlist <testlist.ron>            Run a testlist
testlist --new <path>              Create a new testlist template
testlist --version                 Print version
testlist --help                    Print help

Options:
    --tester <name>    Set tester name (default: $USER)
    --results <path>   Custom results file path
                       (default: <testlist>.results.ron)
    --continue         Continue from existing results
```

### Examples

```bash
testlist ./release-checklist.testlist.ron
testlist ./tests.ron --tester alice --results ./alice-results.ron
testlist ./tests.ron --continue
```

## RON File Format

### Testlist definition (`*.testlist.ron`)

```ron
Testlist(
    meta: Meta(
        title: "My App v0.1.0 Release Checklist",
        description: "Manual verification before release",
        created: "2025-01-24T10:00:00Z",
        version: "1",
    ),
    tests: [
        Test(
            id: "build",
            title: "Application builds successfully",
            description: "Verify the release build completes without errors.",
            setup: [],
            action: "Run the release build",
            verify: [
                ChecklistItem(id: "v0", text: "Build completes without errors"),
                ChecklistItem(id: "v1", text: "Binary is created in target/release/"),
            ],
            suggested_command: Some("cargo build --release"),
        ),
    ],
)
```

> **Note:** Plain strings in `setup` and `verify` arrays are also accepted for backward compatibility.

### Results file (`*.testlist.results.ron`)

Results are written automatically when you quit. Only status is required — notes, screenshots, and sub-checklists are optional.

```ron
Results(
    meta: ResultsMeta(
        testlist: "example.testlist.ron",
        tester: "alice",
        started: "2025-01-24T14:30:00Z",
        completed: Some("2025-01-24T15:45:00Z"),
    ),
    results: [
        TestResult(
            test_id: "build",
            status: Passed,
            notes: Some("Build took 45 seconds."),
            screenshots: [],
            completed_at: Some("2025-01-24T14:32:00Z"),
            setup_checked: Some([]),
            verify_checked: Some([true, true]),
        ),
    ],
)
```

## Keyboard Shortcuts

### Navigation

| Key | Action |
|-----|--------|
| `j/k` or `↑/↓` | Navigate test list (headers only) |
| `Enter`, `l`, or `Space` | Expand/collapse test details |
| `Tab` | Cycle pane focus (Tests → Notes → Terminal) |

### Status Marking

| Key | Action |
|-----|--------|
| `p` | Mark as Passed |
| `f` | Mark as Failed |
| `i` | Mark as Inconclusive |
| `s` | Mark as Skipped |

### Notes & Terminal

| Key | Action |
|-----|--------|
| `n` | Edit notes for current test |
| `a` | Add screenshot path |
| `c` | Insert suggested command into terminal |
| `Esc` | Exit terminal focus / save notes |

### Other

| Key | Action |
|-----|--------|
| `w` | Save results |
| `t` | Toggle theme (dark/light) |
| `?` | Show help popup |
| `q` | Quit (selectable Yes/No dialog if unsaved changes) |

## Workflows

### With Claude Code

1. Claude Code generates a testlist based on changes made
2. Claude instructs: "Run `testlist ./changes.testlist.ron` to verify"
3. User works through items in the TUI
4. User quits — results are saved
5. Claude reads the results file and responds to findings

### Standalone

1. Create a testlist file manually or with `--new`
2. Run `testlist ./checklist.testlist.ron`
3. Work through items at your own pace
4. Results saved for review or documentation

### Quick Pass

1. Run through tests quickly
2. Mark each as pass/fail without notes
3. Only add detail on failures worth explaining
4. Still produces valid, machine-readable results

## License

MIT — Copyright (c) 2026 Nathan Strange. See [LICENSE](LICENSE).
