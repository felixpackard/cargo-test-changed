# cargo-test-changed

A Cargo subcommand to run tests for changed crates and their dependents.

## Features

- Detect changed files using Git, either uncommitted changes or via Git history
- Identify affected crates based on changed files
- Run tests for changed crates, and optionally dependent crates
- Configurable test runner
- JSON output mode for machine consumption
- Re-run tests for failed crates

## Installation

```bash
cargo install --path .
```

## Usage

### Basic usage

```bash
cargo test-changed
```

### Options

- `--changes <MODE>`: Compare changes between VCS references instead of uncommitted changes [default: working] [possible values: working, refs]
- `--from <FROM>`: Starting reference point for comparison (required when using --changes)
- `--to <TO>`: Ending reference point (defaults to current state when using --changes)
- `-r <TEST_RUNNER>`: Specify a custom test runner
- `-d, --with-dependents`: Include tests for crates dependent on the changed crates in the test run
- `-n, --dry-run`: Skip running tests, only print the crates that would be tested
- `-v, --verbose`: Display full output while running tests
- `-k, --no-fail-fast`: Run tests for all crates regardless of failure
- `-c, --crates <CRATES>`: Specify a set of crates to run tests for, typically for re-running failed tests
- `-j, --json`: Output in JSON format for machine consumption
- `-h, --help`: Print help (see more with '--help')
- `-V, --version`: Print version
- `-- <TEST_RUNNER_ARGS>...`: Additional arguments to pass to the test runner

### Examples

```bash
# Run tests for uncommitted changes
cargo test-changed

# Run tests for changes between Git references
cargo test-changed --changes refs --from main --to HEAD

# Run tests for changes between branches
cargo test-changed --changes refs --from release-1.0 --to main

# Run tests for changed crates and their dependents
cargo test-changed --with-dependents

# Use a custom test runner (nextest)
cargo test-changed -r nextest

# Dry run to see which crates would be tested
cargo test-changed --dry-run

# Pass additional arguments to the test runner
cargo test-changed -- --release --all-features

# Generate JSON output
cargo test-changed --json

# Don't stop on first test failure
cargo test-changed --no-fail-fast

# Re-run tests for specific crates
cargo test-changed --crates crate1,crate2

# Verbose output showing test progress
cargo test-changed --verbose
```

## Limitations

- Currently only supports Git as the version control system
- Currently only supports `cargo` and `nextest` test runners
- Does not support testing crates in parallel
