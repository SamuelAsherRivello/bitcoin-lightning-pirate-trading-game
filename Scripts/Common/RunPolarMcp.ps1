$ErrorActionPreference = "Stop"

$targetScript = Join-Path $PSScriptRoot "..\Other\RunPolarMcp.ps1"
& $targetScript @args
