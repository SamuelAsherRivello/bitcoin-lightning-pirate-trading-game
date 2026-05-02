$ErrorActionPreference = "Stop"

$ScriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Resolve-Path (Join-Path $ScriptRoot "..\..")
Set-Location $ProjectRoot

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
    winget install --id Rustlang.Rustup -e --accept-package-agreements --accept-source-agreements

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
    rustup target add wasm32-unknown-unknown
}

function Ensure-DioxusCli {
    $RequiredDxVersion = "0.7.6"
    $DxCommand = Get-Command dx -ErrorAction SilentlyContinue

    if ($DxCommand) {
        $DxVersionOutput = (& dx --version | Out-String).Trim()
        if ($DxVersionOutput -match "^dioxus\s+0\.7(\.|-|$)") {
            Write-Host "Dioxus CLI is already installed and compatible ($DxVersionOutput)."
            return
        }

        Write-Host "Dioxus CLI is installed but not on a known-compatible 0.7 version: $DxVersionOutput"
        Write-Host "Reinstalling Dioxus CLI $RequiredDxVersion..."
        cargo install dioxus-cli@$RequiredDxVersion --force
        return
    }

    Write-Host "Installing Dioxus CLI $RequiredDxVersion..."
    cargo install dioxus-cli@$RequiredDxVersion
}

function Ensure-NodeDependencies {
    if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
        throw "npm is required for Tailwind CSS. Install Node.js from https://nodejs.org/ and rerun this script."
    }

    if (-not (Test-Path (Join-Path $ProjectRoot "node_modules\.bin\tailwindcss.cmd"))) {
        Write-Host "Installing Tailwind CSS dependencies..."
        npm install
    } else {
        Write-Host "Tailwind CSS dependencies are already installed."
    }
}

function Build-TailwindCss {
    Write-Host "Building Tailwind CSS..."
    npm run tailwind:build
}

Ensure-RustToolchain
Ensure-WasmTarget
Ensure-DioxusCli
Ensure-NodeDependencies
Build-TailwindCss

Write-Host ""
Write-Host "Running validation build..."
cargo check --workspace

Write-Host ""
Write-Host "Dependency install complete."
