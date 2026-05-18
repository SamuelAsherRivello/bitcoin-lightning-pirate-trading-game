param(
    [int]$Port = 8080
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$repoRootPath = $repoRoot.Path
$escapedRepoRoot = [regex]::Escape($repoRootPath)
$script:StoppedProcessIds = @{}

function Stop-ProcessById {
    param(
        [int]$ProcessId,
        [string]$Reason
    )

    if ($ProcessId -le 0 -or $ProcessId -eq $PID -or $script:StoppedProcessIds.ContainsKey($ProcessId)) {
        return
    }

    $process = Get-CimInstance Win32_Process -Filter "ProcessId = $ProcessId" -ErrorAction SilentlyContinue
    if (-not $process) {
        return
    }

    $script:StoppedProcessIds[$ProcessId] = $true
    Write-Host "Stopping $($process.Name) process $ProcessId ($Reason)."
    Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
    Wait-Process -Id $ProcessId -Timeout 5 -ErrorAction SilentlyContinue
}

function Stop-DioxusPortListeners {
    $listeners = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue

    foreach ($listener in $listeners) {
        $process = Get-CimInstance Win32_Process -Filter "ProcessId = $($listener.OwningProcess)" -ErrorAction SilentlyContinue
        $commandLine = if ($process) { $process.CommandLine } else { "" }

        if ($process -and $process.Name -ieq "dx.exe") {
            Stop-ProcessById -ProcessId $listener.OwningProcess -Reason "Dioxus static/hot-reload server listening on port $Port"
        } elseif ($commandLine -match $escapedRepoRoot -and $commandLine -match "\\target\\dx\\web\\") {
            Stop-ProcessById -ProcessId $listener.OwningProcess -Reason "generated app server listening on port $Port"
        }
    }
}

$allProcesses = Get-CimInstance Win32_Process -ErrorAction SilentlyContinue

$dioxusServers = $allProcesses |
    Where-Object {
        $_.Name -ieq "dx.exe" `
            -and $_.CommandLine -match "\bserve\b" `
            -and $_.CommandLine -match "--port(=|\s+)$Port(\s|$)"
    }

$gameServers = $allProcesses |
    Where-Object {
        ($_.Name -like "server-*.exe" -or $_.Name -ieq "web.exe") `
            -and $_.CommandLine -match $escapedRepoRoot `
            -and $_.CommandLine -match "\\target\\dx\\web\\"
    }

$dioxusServers |
    ForEach-Object {
        Stop-ProcessById -ProcessId $_.ProcessId -Reason "Dioxus static/hot-reload server on port $Port"
    }

$gameServers |
    ForEach-Object {
        Stop-ProcessById -ProcessId $_.ProcessId -Reason "generated app server for this repository"
    }

Stop-DioxusPortListeners

for ($attempt = 0; $attempt -lt 10; $attempt += 1) {
    $targetListeners = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue |
        Where-Object {
            $process = Get-CimInstance Win32_Process -Filter "ProcessId = $($_.OwningProcess)" -ErrorAction SilentlyContinue
            $commandLine = if ($process) { $process.CommandLine } else { "" }

            ($process -and $process.Name -ieq "dx.exe") -or
                ($commandLine -match $escapedRepoRoot -and $commandLine -match "\\target\\dx\\web\\")
        }

    if (-not $targetListeners) {
        break
    }

    Stop-DioxusPortListeners
    Start-Sleep -Milliseconds 500
}

$remainingListeners = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue
foreach ($listener in $remainingListeners) {
    $process = Get-CimInstance Win32_Process -Filter "ProcessId = $($listener.OwningProcess)" -ErrorAction SilentlyContinue
    $processName = if ($process) { $process.Name } else { "unknown" }
    $commandLine = if ($process) { $process.CommandLine } else { "" }

    if ($commandLine -match $escapedRepoRoot -and $commandLine -match "\\target\\dx\\web\\") {
        Stop-ProcessById -ProcessId $listener.OwningProcess -Reason "generated app server still listening on port $Port"
    } else {
        throw "Port $Port is already in use by $processName process $($listener.OwningProcess). Stop that process or choose another -Port."
    }
}

if ($script:StoppedProcessIds.Count -eq 0) {
    Write-Host "No web app processes for this repository were running on port $Port."
} else {
    Write-Host "Stopped $($script:StoppedProcessIds.Count) web app process(es) for this repository."
}
