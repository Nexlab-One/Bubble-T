$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $Root

git config core.hooksPath .githooks

Write-Host "Git hooks installed (core.hooksPath=.githooks)"
Write-Host "  pre-commit: cargo fmt + clippy"
Write-Host "  pre-push:   cargo fmt --check + clippy"
