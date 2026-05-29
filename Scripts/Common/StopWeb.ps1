$ErrorActionPreference = "Stop"

$targetScript = Join-Path $PSScriptRoot "..\Other\StopWeb.ps1"
& $targetScript @args
