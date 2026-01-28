# testlist

**A TUI tool for structured human feedback collection**

## Overview

testlist is a terminal-based tool that guides users through manual testing, evaluation, and feedback tasks. It displays a checklist of items to verify, provides an embedded terminal for running commands, and captures structured notes and screenshots.

While automated tests verify *what code does*, testlist captures *what humans observe* - subjective quality, usability, edge cases, and anything that requires human judgment.

### Philosophy

- **Human judgment is valuable** - Some things can't be automated: "Does this feel right?" "Is this confusing?" "What happens if you try to break it?"
- **Structure reduces cognitive load** - Checklists with clear setup/action/verify steps help tired developers do thorough work
- **Results should be machine-readable** - Output is consumable by Claude Code, scripts, or other tools
- **Simple first** - Start minimal, extend later
- **Let users be lazy** - Status-only feedback is valid; notes and screenshots are optional

### Use Cases

- Manual verification of features Claude Code built
- UX and accessibility evaluation
- Documentation testing ("Follow these steps. Did they work?")
- Cross-platform testing on machines Claude can't access
- Stakeholder review and sign-off
- Exploratory testing ("Try to break this")
- Multi-user feedback collection (roadmap)

---

## Core Concepts

### Testlist File

A RON file defining what the user should test or evaluate. Contains metadata and an ordered list of test items.

### Test Item

A single thing to verify. Can be objective ("Does it compile?") or subjective ("Is the onboarding flow intuitive?"). Contains:

- **Title** - Brief description
- **Description** - Detailed instructions (markdown)
- **Setup steps** - Prerequisites checklist (optional)
- **Action** - What the user should do
- **Verify steps** - What to check afterward (optional)
- **Suggested command** - Optional command to run (tab-completable in terminal)

### Results

User-provided feedback for each test item. Only status is required:

- **Status** - `pending`, `passed`, `failed`, `inconclusive`, `skipped` *(required)*
- **Notes** - Free-form observations *(optional)*
- **Screenshots** - File paths to captured images *(optional)*
- **Sub-checklists** - Setup/verify step states *(optional, defaults unchecked)*

---

## RON Schema

### Testlist Definition

```ron
// example.testlist.ron
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
                "Build completes without errors",
                "Binary is created in target/release/",
            ],
            suggested_command: Some("cargo build --release"),
        ),
        Test(
            id: "login-flow",
            title: "User login flow",
            description: r#"
Test the complete login experience.

Pay attention to:
- Loading states
- Error messages
- Keyboard navigation
            "#,
            setup: [
                "Start the dev server",
                "Open browser to http://localhost:3000",
                "Ensure test user exists (user: demo, pass: demo)",
            ],
            action: "Attempt to log in with the test credentials",
            verify: [
                "Login button shows loading state",
                "Redirected to /dashboard on success",
                "Welcome message displays username",
                "Session persists on page refresh",
            ],
            suggested_command: Some("cargo run --bin server"),
        ),
        Test(
            id: "ux-review",
            title: "Overall UX impression",
            description: r#"
Spend 5 minutes using the app freely.

Consider:
- Is navigation intuitive?
- Are actions discoverable?
- Does anything feel slow or janky?
            "#,
            setup: [],
            action: "Explore the application",
            verify: [
                "Navigation is intuitive",
                "No confusing dead-ends",
                "Performance feels acceptable",
            ],
            suggested_command: None,
        ),
    ],
)
```

### Results File

```ron
// example.testlist.results.ron
Results(
    meta: ResultsMeta(
        testlist: "example.testlist.ron",
        tester: "alice",
        started: "2025-01-24T14:30:00Z",
        completed: Some("2025-01-24T15:45:00Z"),
    ),
    results: [
        // Thorough tester: filled everything out
        TestResult(
            test_id: "build",
            status: Passed,
            notes: Some("Build took 45 seconds, seems reasonable."),
            screenshots: [],
            completed_at: Some("2025-01-24T14:32:00Z"),
            setup_checked: Some([]),
            verify_checked: Some([true, true]),
        ),
        // Detailed failure: notes and screenshots
        TestResult(
            test_id: "login-flow",
            status: Failed,
            notes: Some("Login button takes ~3 seconds to respond. Error message on wrong password is vague ('An error occurred')."),
            screenshots: [
                "/home/alice/screenshots/login-slow.png",
                "/home/alice/screenshots/error-message.png",
            ],
            completed_at: Some("2025-01-24T14:50:00Z"),
            setup_checked: Some([true, true, true]),
            verify_checked: Some([true, true, true, false]),
        ),
        // Lazy but valid: just status
        TestResult(
            test_id: "ux-review",
            status: Passed,
            notes: None,
            screenshots: [],
            completed_at: Some("2025-01-24T15:10:00Z"),
            setup_checked: None,
            verify_checked: None,
        ),
    ],
)
```

---

## TUI Layout

```
┌─ Tests ──────────────────────────────────┬─ Notes ─────────────────────────┐
│ [✓] Build successfully                   │ Login button takes ~3 seconds   │
│ [✗] Login flow                           │ to respond. Error message on    │
│ [?] Overall UX impression                │ wrong password is vague.        │
│ [ ] Error handling                       │                                 │
│                                          │ Screenshots:                    │
│ ▼ [✗] Login flow                         │   [1] login-slow.png            │
│   ┌ Setup ────────────────────────────┐  │   [2] error-message.png         │
│   │ [✓] Start dev server              │  │   [+] Add screenshot...         │
│   │ [✓] Open browser                  │  │                                 │
│   │ [✓] Test user exists              │  │                                 │
│   └───────────────────────────────────┘  │                                 │
│   Action: Attempt to log in with the     │                                 │
│           test credentials               │                                 │
│   ┌ Verify ───────────────────────────┐  │                                 │
│   │ [✓] Loading state shown           │  │                                 │
│   │ [✓] Redirected                    │  │                                 │
│   │ [✓] Welcome message               │  │                                 │
│   │ [ ] Session persists              │  │                                 │
│   └───────────────────────────────────┘  │                                 │
├─ Terminal ───────────────────────────────┴─────────────────────────────────┤
│ $ cargo run --bin server                                                   │
│    Compiling myapp v0.1.0                                                  │
│    Finished dev [unoptimized + debuginfo] target(s) in 2.34s               │
│    Running `target/debug/server`                                           │
│ Server running on http://localhost:3000                                    │
│ $ _                                                                        │
├────────────────────────────────────────────────────────────────────────────┤
│ [P]ass [F]ail [I]nconclusive [S]kip │ [Tab] Pane │ [↑↓] History │ [Q]uit  │
└────────────────────────────────────────────────────────────────────────────┘
```

### Panes

| Pane | Purpose |
|------|---------|
| **Tests** | Collapsible tree of test items with sub-checklists |
| **Notes** | Free-form text entry and screenshot list for current test |
| **Terminal** | Embedded PTY for running commands (full width for long commands) |
| **Status Bar** | Keyboard shortcuts and progress summary |

### Key Interactions

| Key | Context | Action |
|-----|---------|--------|
| `↑/↓` or `j/k` | Tests pane | Navigate test list |
| `↑/↓` | Terminal pane | Cycle through command history |
| `Tab` | Terminal pane | Auto-complete suggested commands for current test |
| `Enter` or `l` | Tests pane | Expand/collapse test item |
| `Space` | Tests pane | Toggle sub-checklist item |
| `Tab` | Global | Cycle focus between panes |
| `p` | Tests pane | Mark current test Passed |
| `f` | Tests pane | Mark current test Failed |
| `i` | Tests pane | Mark current test Inconclusive |
| `s` | Tests pane | Mark current test Skipped |
| `n` | Global | Focus notes pane for editing |
| `a` | Notes pane | Add screenshot (prompts for path) |
| `Esc` | Notes pane | Exit editing mode |
| `q` | Global | Quit (with confirmation if incomplete) |
| `Ctrl+s` | Global | Save results |

### Terminal Tab Completion

When focused in the terminal pane, pressing `Tab` on an empty or partial line offers completion from:

1. **Suggested command** for the currently selected test (highest priority)
2. **All suggested commands** from the testlist
3. Standard shell completion (if PTY supports it)

Example:
```
$ car<Tab>
→ $ cargo run --bin server    (suggested for "Login flow" test)
```

---

## Workflows

### With Claude Code

1. Claude Code generates a testlist file based on changes made
2. Claude Code instructs user: `Run 'testlist ./changes.testlist.ron' to verify`
3. User runs testlist, works through items
4. User quits testlist when done
5. Claude Code reads results file and responds to findings

### Standalone

1. User (or team lead) creates testlist file manually
2. User runs `testlist ./checklist.testlist.ron`
3. User works through items at their own pace
4. Results saved for review or documentation

### Quick Pass (Lazy Mode)

1. User runs through tests quickly
2. Marks each as pass/fail without notes
3. Only adds detail on failures worth explaining
4. Still produces valid, machine-readable results

### Multi-User (Roadmap)

1. Testlist file committed to repo
2. Each tester runs testlist, results saved to `*.results.<username>.ron`
3. Each tester commits their results file
4. Aggregation tool (future) or manual review combines feedback

---

## CLI Interface

```
testlist - Structured human feedback collection

USAGE:
    testlist <testlist.ron>
    testlist --new <output.ron>
    testlist --version
    testlist --help

ARGS:
    <testlist.ron>    Path to testlist definition file

OPTIONS:
    --new <path>      Create a new testlist template
    --tester <name>   Set tester name for results (default: $USER)
    --results <path>  Custom path for results file
                      (default: <testlist>.results.ron)
    --continue        Continue from existing results file
    -h, --help        Print help
    -V, --version     Print version

EXAMPLES:
    testlist ./release-checklist.testlist.ron
    testlist --new ./my-tests.testlist.ron
    testlist ./tests.ron --tester alice --results ./alice-results.ron
```

---

## Development Roadmap

### Phase 1: Core Functionality

- [x] RON schema definition and serde parsing
- [x] Basic TUI layout with ratatui (tests + notes + terminal panes)
- [x] Test list display with expand/collapse
- [x] Sub-checklist toggling (optional - can skip)
- [x] Status marking (pass/fail/inconclusive/skipped)
- [x] Notes text entry (optional)
- [x] Screenshot path entry (optional)
- [x] Results file writing (status always saved, rest if provided)
- [x] CLI argument parsing

### Phase 2: Terminal Integration

- [x] Embedded PTY in terminal pane (bottom, full width)
- [ ] Command history with up/down arrows
- [ ] Tab completion for suggested commands
- [ ] Copy/paste support

### Phase 3: Polish

- [ ] Markdown syntax highlighting in descriptions
- [x] Progress indicator (3/10 complete)
- [x] Quit confirmation with unsaved changes
- [ ] Auto-save on test completion
- [x] Configurable colors/theme
- [x] Resize handling

### Phase 4: Enhanced Screenshots

- [ ] Platform-specific paste support (macOS pbpaste for file paths)
- [ ] Watch directory for new screenshots
- [ ] Thumbnail preview (sixel/kitty protocol for supported terminals)

### Phase 5: Multi-User Support

- [x] `--tester` flag for identifying user
- [ ] Results file naming convention
- [ ] `testlist aggregate <dir>` command to summarize multiple results
- [ ] Conflict-free design for git/jj workflows

### Future Ideas

- tmux integration (Claude Code launches testlist in split pane)
- Web viewer for results (read-only HTML report)
- Test templates library
- Integration with issue trackers (create issues from failed tests)
- Voice notes (audio file attachment)
- Time tracking per test item

---

## File Locations

| File | Purpose |
|------|---------|
| `*.testlist.ron` | Test definitions (can be version controlled) |
| `*.testlist.results.ron` | Single-user results (gitignored or committed per workflow) |
| `*.testlist.results.<user>.ron` | Multi-user results (Phase 5) |

---

## Design Decisions

**Why RON?**
- Native Rust ecosystem (serde support)
- Human-readable and editable
- Supports multiline strings cleanly
- Less ambiguous than YAML

**Why separate results file?**
- Keeps test definitions immutable during testing
- Enables multi-user without merge conflicts on definitions
- Clear separation of "what to test" vs "what happened"

**Why bottom terminal pane?**
- Commands are often long; width matters more than visible history
- Matches common IDE layouts (editor above, terminal below)
- History accessible via up/down arrows, doesn't need visual space

**Why PTY instead of simple command execution?**
- Interactive commands (vim, htop, etc.) work correctly
- Proper terminal rendering (colors, cursor movement)
- Matches user expectations for a "terminal pane"
- Worth the complexity for the UX improvement

**Why optional everything except status?**
- Respects user time and energy
- "Passed" with no notes is still useful signal
- Encourages quick passes through easy tests
- Detailed feedback where it matters (failures, surprises)

**Why sub-checklists?**
- Reduces cognitive load ("what was I supposed to check?")
- Structured data for potential automation/analysis later
- Helps tired developers be thorough
- But optional - power users can skip them
