$ErrorActionPreference = "Stop"

$ScriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Resolve-Path (Join-Path $ScriptRoot "..\..")
Set-Location $ProjectRoot

if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    throw "Node.js 18 or newer is required for @lightningpolar/mcp. Install Node.js from https://nodejs.org/ and rerun this script."
}

$NodeVersionText = (& node --version).Trim().TrimStart("v")
$NodeMajor = [int]($NodeVersionText.Split(".")[0])
if ($NodeMajor -lt 18) {
    throw "Node.js 18 or newer is required for @lightningpolar/mcp. Current node version: v$NodeVersionText."
}

if (-not (Get-Command npx -ErrorAction SilentlyContinue)) {
    throw "npx is required to launch @lightningpolar/mcp. Install Node.js/npm from https://nodejs.org/ and rerun this script."
}

Write-Host "Checking Polar local bridge at http://localhost:37373/health..."
try {
    $Health = Invoke-RestMethod -Uri "http://localhost:37373/health" -TimeoutSec 3
    if ($Health.status) {
        Write-Host "Polar bridge status: $($Health.status)"
    } else {
        Write-Host "Polar bridge responded."
    }
} catch {
    Write-Host "Polar bridge did not respond yet. Start Polar before running networked setup."
}

Write-Host "Starting Polar MCP helper with npx -y @lightningpolar/mcp..."
Write-Host "Leave this terminal open while using the app's Polar Connection (Networked) setup."
& npx -y @lightningpolar/mcp
