# OPTS Local CI Script (PowerShell for Windows)
# Runs code formatting, linting, tests, documentation checks, and mdBook checks.
# Exit code: 0 on success, non-zero on failure.

$ErrorActionPreference = "Continue"

$RootDir = Split-Path -Parent $PSScriptRoot
Set-Location $RootDir

Write-Host "=== Running Local CI ===" -ForegroundColor Cyan

$statusFmt = "pending"
$statusClippy = "pending"
$statusTest = "pending"
$statusDoc = "pending"
$statusMdbook = "pending"
$failed = $false

# 1. Format Check
Write-Host ""
Write-Host "--- Checking formatting (cargo fmt) ---" -ForegroundColor Yellow
cargo fmt --all -- --check
if ($LASTEXITCODE -eq 0) {
    $statusFmt = "✓"
} else {
    $statusFmt = "✗"
    $failed = $true
}

# 2. Clippy
Write-Host ""
Write-Host "--- Running Clippy (cargo clippy) ---" -ForegroundColor Yellow
cargo clippy --workspace --all-targets --all-features -- -D warnings
if ($LASTEXITCODE -eq 0) {
    $statusClippy = "✓"
} else {
    $statusClippy = "✗"
    $failed = $true
}

# 3. Tests
Write-Host ""
Write-Host "--- Running tests (cargo test) ---" -ForegroundColor Yellow
cargo test --workspace --all-features
if ($LASTEXITCODE -eq 0) {
    $statusTest = "✓"
} else {
    $statusTest = "✗"
    $failed = $true
}

# 4. Doc
Write-Host ""
Write-Host "--- Checking rustdoc (cargo doc) ---" -ForegroundColor Yellow
$env:RUSTDOCFLAGS = "-D warnings"
cargo doc --workspace --no-deps
if ($LASTEXITCODE -eq 0) {
    $statusDoc = "✓"
} else {
    $statusDoc = "✗"
    $failed = $true
}

# 5. mdBook
Write-Host ""
Write-Host "--- Checking mdBook documentation ---" -ForegroundColor Yellow
if (Get-Command mdbook -ErrorAction SilentlyContinue) {
    mdbook build docs
    if ($LASTEXITCODE -eq 0) {
        $statusMdbook = "✓"
    } else {
        $statusMdbook = "✗"
        $failed = $true
    }
} else {
    Write-Warning "mdbook is not installed; skipping mdBook build"
    $statusMdbook = "skipped (not installed)"
}

# 6. Roadmap and architecture documents (M-0074)
Write-Host ""
Write-Host "--- Checking roadmap copies, roadmap drift and the crate graph ---" -ForegroundColor Yellow
if (Get-Command python3 -ErrorAction SilentlyContinue) {
    $roadmapOk = $true
    foreach ($check in @("gen_roadmap_copy.py", "roadmap_audit.py", "crate_graph.py", "omsi_uses.py")) {
        python3 "scripts/$check" --check
        if ($LASTEXITCODE -ne 0) { $roadmapOk = $false }
    }
    if ($roadmapOk) {
        $statusRoadmap = "✓"
    } else {
        $statusRoadmap = "✗"
        $failed = $true
    }
} else {
    Write-Warning "python3 is not installed; skipping the roadmap checks"
    $statusRoadmap = "skipped (no python3)"
}

Write-Host ""
Write-Host "=== Local CI Summary ===" -ForegroundColor Cyan
Write-Host "  fmt:      $statusFmt"
Write-Host "  clippy:   $statusClippy"
Write-Host "  test:     $statusTest"
Write-Host "  doc:      $statusDoc"
Write-Host "  mdbook:   $statusMdbook"
Write-Host "  roadmap:  $statusRoadmap"

if ($failed) {
    Write-Host ""
    Write-Host "Local CI FAILED" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "Local CI PASSED" -ForegroundColor Green
exit 0
