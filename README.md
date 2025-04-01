# cargo-test-changed

A Cargo subcommand to run tests for changed crates and their dependents.

## Features

- Detect changed files using Git
- Identify affected crates based on changed files
- Run tests for changed crates and their dependents
- Configurable test runner
- JSON output mode for machine consumption
- Re-run tests for failed crates

## Installation

```bash
cargo install --path .
```

## Usage

Basic usage:
```bash
cargo test-changed
```

Options:
- `-t`, `--test-runner`: Specify a custom test runner
- `-w`, `--with-dependents`: Include tests for crates dependent on the changed crates in the test run
- `-d`, `--dry-run`: Skip running tests, only print the crates that would be tested
- `-v`, `--verbose`: Display full output while running tests
- `-n`, `--no-fail-fast`: Run tests for all crates regardless of failure
- `-j`, `--json`: Output in JSON format for machine consumption
- `-c`, `--crates`: Specify a set of crates to run tests for, typically for re-running failed tests
- `-- <TEST_RUNNER_ARGS>...`: Additional arguments to pass to the test runner

Examples:
```bash
# Run tests for changed crates and their dependents
cargo test-changed

# Run tests for changed crates only
cargo test-changed --skip-dependents

# Use a custom test runner
cargo test-changed --test-runner nextest

# Pass additional arguments to the test runner
cargo test-changed -- --release
```

## Limitations

- Currently only supports Git as the version control system
- Currently only detects changes based on your Git status, not your Git history
- Currently only supports `cargo` and `nextest` test runners
- Does not support testing crates in parallel
