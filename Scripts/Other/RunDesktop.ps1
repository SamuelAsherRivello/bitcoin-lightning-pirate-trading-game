$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $repoRoot

function Stop-ProcessById {
    param(
        [int]$ProcessId,
        [string]$Reason
    )

    if ($ProcessId -le 0 -or $ProcessId -eq $PID) {
        return
    }

    $process = Get-CimInstance Win32_Process -Filter "ProcessId = $ProcessId" -ErrorAction SilentlyContinue
    if (-not $process) {
        return
    }

    Write-Host "Stopping $($process.Name) process $ProcessId ($Reason)."
    Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
    Wait-Process -Id $ProcessId -Timeout 5 -ErrorAction SilentlyContinue
}

Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -ieq "dx.exe" -and $_.CommandLine -match "\bserve\b" } |
    ForEach-Object {
        Stop-ProcessById -ProcessId $_.ProcessId -Reason "existing Dioxus server"
    }

if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
    throw "npm is required for Tailwind CSS. Run Scripts\Common\InstallDependencies.ps1 first."
}

Write-Host "Building Tailwind CSS..."
npm run tailwind:build

dx serve --package desktop --desktop
