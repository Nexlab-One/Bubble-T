#!/usr/bin/env sh
set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

git config core.hooksPath .githooks

if [ "$(uname -s)" != "MINGW"* ] && [ "$(uname -s)" != "MSYS"* ] && [ "$(uname -s)" != "CYGWIN"* ]; then
  chmod +x .githooks/pre-commit .githooks/pre-push scripts/rust-checks.sh
fi

echo "Git hooks installed (core.hooksPath=.githooks)"
echo "  pre-commit: cargo fmt + clippy"
echo "  pre-push:   cargo fmt --check + clippy"
