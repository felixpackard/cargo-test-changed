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
- `--test-runner`: Specify a custom test runner
- `--skip-dependents`: Skip dependent crates, only test crates with changes
- `--dry-run`: Skip running tests, only print the crates that would be tested
- `--verbose`: Display full output while running tests

Examples:
```bash
# Run tests for changed crates and their dependents
cargo test-changed

# Run tests for changed crates only
cargo test-changed --skip-dependents

# Use a custom test runner
cargo test-changed --test-runner nextest
```
