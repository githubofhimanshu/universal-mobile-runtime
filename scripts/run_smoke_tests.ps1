<#
.SYNOPSIS
    Runs the complete Universal Mobile Runtime (UMR) Phase 1 Vertical Slice smoke test suite.
.DESCRIPTION
    Builds all workspace crates, executes unit and integration tests, and runs the Gate 1 acceptance test suite.
#>

$ErrorActionPreference = "Stop"

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host " UMR Phase 1 Vertical Slice Smoke Test Runner" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan

# Ensure cargo is on PATH
$cargoBin = "$env:USERPROFILE\.cargo\bin"
if (Test-Path "$cargoBin\cargo.exe") {
    $env:PATH = "$cargoBin;$env:PATH"
}

Write-Host "[1/3] Building workspace crates..." -ForegroundColor Yellow
cargo build --workspace
if ($LASTEXITCODE -ne 0) {
    Write-Error "Cargo build failed"
    exit 1
}

Write-Host "[2/3] Running full test suite..." -ForegroundColor Yellow
cargo test --workspace
if ($LASTEXITCODE -ne 0) {
    Write-Error "Cargo test suite failed"
    exit 1
}

Write-Host "[3/3] Running Gate 1 Acceptance Criteria (AC-1 .. AC-7)..." -ForegroundColor Yellow
cargo test -p umr-orchestrator --test e2e_smoke -- --nocapture
if ($LASTEXITCODE -ne 0) {
    Write-Error "Gate 1 smoke tests failed"
    exit 1
}

Write-Host "============================================================" -ForegroundColor Green
Write-Host " ALL PHASE 1 SMOKE TESTS & ACCEPTANCE CRITERIA PASSED" -ForegroundColor Green
Write-Host "============================================================" -ForegroundColor Green

