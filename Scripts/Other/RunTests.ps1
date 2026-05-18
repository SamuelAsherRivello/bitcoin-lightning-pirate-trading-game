$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $repoRoot

cargo test -p ui --lib --test tests
.\Scripts\Other\RunScriptTests.ps1
