#!/usr/bin/env sh
# Shared Rust quality checks used by git hooks.
# Usage: rust-checks.sh [fix|check]
#   fix   — format in place (pre-commit)
#   check — verify formatting only (pre-push)

set -eu

FMT_MODE="${1:-check}"

if [ "$FMT_MODE" = "fix" ]; then
  echo "cargo fmt --all"
  cargo fmt --all
else
  echo "cargo fmt --all -- --check"
  cargo fmt --all -- --check
fi

echo "cargo clippy --workspace --all-targets --all-features -- -D warnings"
cargo clippy --workspace --all-targets --all-features -- -D warnings
