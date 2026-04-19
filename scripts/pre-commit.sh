#!/bin/bash
# Pre-commit hook to run cargo fmt and cargo clippy

set -ueE -o pipefail

# Colors for output
GREEN=$'\033[0;32m'
RED=$'\033[0;31m'
NC=$'\033[0m' # No Color

panic() {
    echo "${RED}ERROR:${NC} $*" >&2
    exit 1
}

info() {
    echo "${GREEN}INFO:${NC} $*"
}

info "Checking code formatting with cargo fmt..."
cargo fmt --all -- --check || panic "Formatting issues found. Run 'cargo fmt' to fix them."

info "Running lints with cargo clippy..."
cargo clippy --all-targets --all-features -- -D warnings || panic "Clippy found issues. Please fix them before committing."

info "Running tests..."
cargo test || panic "Tests failed. Please fix them before committing."

info "Pre-commit checks passed!"
exit 0
