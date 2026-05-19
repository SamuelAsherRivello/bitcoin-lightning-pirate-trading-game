$ErrorActionPreference = "Stop"

$ScriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Resolve-Path (Join-Path $ScriptRoot "..\..")
Set-Location $ProjectRoot

function Invoke-CheckedCommand {
    param(
        [string]$FilePath,
        [string[]]$ArgumentList
    )

    & $FilePath @ArgumentList
    if ($LASTEXITCODE -ne 0) {
        throw "$FilePath exited with code $LASTEXITCODE."
    }
}

function Add-CargoToPathIfPresent {
    $CargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    if ((Test-Path $CargoBin) -and (-not (($env:Path -split ";") -contains $CargoBin))) {
        $env:Path = "$CargoBin;$env:Path"
    }
}

function Ensure-RustToolchain {
    Add-CargoToPathIfPresent

    if (Get-Command cargo -ErrorAction SilentlyContinue) {
        Write-Host "Rust is already installed."
        return
    }

    if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
        throw "Rust is not installed and winget is unavailable. Install rustup from https://rustup.rs/ and rerun this script."
    }

    Write-Host "Installing Rust via rustup..."
    Invoke-CheckedCommand "winget" @("install", "--id", "Rustlang.Rustup", "-e", "--accept-package-agreements", "--accept-source-agreements")

    Add-CargoToPathIfPresent

    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "Rust installation finished, but cargo is not available in this PowerShell session yet. Open a new terminal and rerun this script."
    }
}

function Ensure-WasmTarget {
    $InstalledTargets = rustup target list --installed
    if ($InstalledTargets -contains "wasm32-unknown-unknown") {
        Write-Host "wasm32-unknown-unknown target is already installed."
        return
    }

    Write-Host "Installing wasm32-unknown-unknown target..."
    Invoke-CheckedCommand "rustup" @("target", "add", "wasm32-unknown-unknown")
}

function Get-WorkspaceDioxusVersion {
    $CargoTomlPath = Join-Path $ProjectRoot "Cargo.toml"
    $CargoToml = Get-Content -Raw -Path $CargoTomlPath

    if ($CargoToml -match '(?m)^\s*dioxus\s*=\s*\{[^}]*version\s*=\s*"([^"]+)"') {
        return $Matches[1]
    }

    throw "Could not find the workspace Dioxus version in $CargoTomlPath."
}

function Ensure-DioxusCli {
    $RequiredDxVersion = Get-WorkspaceDioxusVersion
    $ExpectedDxVersionPattern = "^dioxus\s+$([regex]::Escape($RequiredDxVersion))(\s|\(|$)"
    $DxCommand = Get-Command dx -ErrorAction SilentlyContinue

    if ($DxCommand) {
        $DxVersionOutput = (& dx --version | Out-String).Trim()
        if ($DxVersionOutput -match $ExpectedDxVersionPattern) {
            Write-Host "Dioxus CLI is already installed and compatible ($DxVersionOutput)."
            return
        }

        Write-Host "Dioxus CLI version does not match workspace Dioxus $RequiredDxVersion ($DxVersionOutput)."
        Write-Host "Reinstalling Dioxus CLI $RequiredDxVersion..."
        Invoke-CheckedCommand "cargo" @("install", "dioxus-cli@$RequiredDxVersion", "--locked", "--force")
        return
    }

    Write-Host "Installing Dioxus CLI $RequiredDxVersion..."
    Invoke-CheckedCommand "cargo" @("install", "dioxus-cli@$RequiredDxVersion", "--locked", "--force")
}

function Ensure-NodeDependencies {
    if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
        throw "npm is required for Tailwind CSS. Install Node.js from https://nodejs.org/ and rerun this script."
    }

    if (-not (Test-Path (Join-Path $ProjectRoot "node_modules\.bin\tailwindcss.cmd"))) {
        Write-Host "Installing Tailwind CSS dependencies..."
        Invoke-CheckedCommand "npm" @("install")
    } else {
        Write-Host "Tailwind CSS dependencies are already installed."
    }
}

function Ensure-PolarMcpPrerequisites {
    if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
        throw "Node.js 18 or newer is required for the Polar MCP helper. Install Node.js from https://nodejs.org/ and rerun this script."
    }

    $NodeVersionText = (& node --version).Trim().TrimStart("v")
    $NodeMajor = [int]($NodeVersionText.Split(".")[0])
    if ($NodeMajor -lt 18) {
        throw "Node.js 18 or newer is required for the Polar MCP helper. Current node version: v$NodeVersionText."
    }

    if (-not (Get-Command npx -ErrorAction SilentlyContinue)) {
        throw "npx is required to launch @lightningpolar/mcp. Install Node.js/npm from https://nodejs.org/ and rerun this script."
    }

    Write-Host "Polar MCP prerequisites are available. Start the helper with .\Scripts\Common\RunPolarMcp.ps1 when using networked Polar setup."
}

function Build-TailwindCss {
    Write-Host "Building Tailwind CSS..."
    Invoke-CheckedCommand "npm" @("run", "tailwind:build")
}

Ensure-RustToolchain
Ensure-WasmTarget
Ensure-DioxusCli
Ensure-NodeDependencies
Ensure-PolarMcpPrerequisites
Build-TailwindCss

Write-Host ""
Write-Host "Running validation build..."
Invoke-CheckedCommand "cargo" @("check", "--workspace")

Write-Host ""
Write-Host "Dependency install complete."
