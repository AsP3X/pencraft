#!/usr/bin/env bash
# Human: The full quality gate (definition-of-done.md): format, clippy with warnings as errors, tests, and the
# dead-code grep that a cfg_attr can't bypass. Run from anywhere.
# Agent: EXITS non-zero on the first failing step; READS nothing outside the workspace.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
if grep -rnE '(allow|expect)\((.*,\s*)?dead_code' src crates; then
  echo "dead_code must not be silenced (rust-no-dead-code-allow.md)" >&2
  exit 1
fi
echo "check: all passed"
