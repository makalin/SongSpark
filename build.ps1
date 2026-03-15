# SongSpark build script
# Requires: Rust (stable), Trunk (cargo install trunk)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

# examples/, samples/, and packs/ are copied into dist by Trunk (see index.html copy-dir). No static folder needed.

# Prefer trunk on PATH; fallback to cargo bin
$trunk = Get-Command trunk -ErrorAction SilentlyContinue
if (-not $trunk) {
    $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin\trunk.exe"
    if (Test-Path $cargoBin) {
        & $cargoBin build
        exit $LASTEXITCODE
    }
    Write-Host "Trunk not found. Install with: cargo install trunk"
    exit 1
}

trunk build --public-url "./"
exit $LASTEXITCODE
