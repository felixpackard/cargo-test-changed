# cargo-test-changed

A Cargo subcommand to run tests for changed crates and their dependents.

## Features

- Detect changed files using Git
- Identify affected crates based on changed files
- Run tests for changed crates and their dependents
- Configurable test runner

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
- `-s`, `--skip-dependents`: Skip dependent crates, only test crates with changes
- `-d`, `--dry-run`: Skip running tests, only print the crates that would be tested
- `-v`, `--verbose`: Display full output while running tests
- `-n`, `--no-fail-fast`: Run tests for all crates regardless of failure
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
- Currently only supports `cargo` and `nextest` test runners
- Does not support running tests in parallel
