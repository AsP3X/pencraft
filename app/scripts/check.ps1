# Human: The full quality gate on Windows, same steps as check.sh: format, clippy (-D warnings), tests, dead-code grep.
# Agent: stops on the first failing step.
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")
cargo fmt --all --check; if ($LASTEXITCODE) { exit $LASTEXITCODE }
cargo clippy --workspace --all-targets -- -D warnings; if ($LASTEXITCODE) { exit $LASTEXITCODE }
cargo test --workspace; if ($LASTEXITCODE) { exit $LASTEXITCODE }
$hits = Get-ChildItem -Recurse -Include *.rs src, crates | Select-String -Pattern '(allow|expect)\((.*,\s*)?dead_code'
if ($hits) { $hits; Write-Error "dead_code must not be silenced (rust-no-dead-code-allow.md)"; exit 1 }
Write-Output "check: all passed"
