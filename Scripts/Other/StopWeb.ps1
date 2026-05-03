param(
    [int]$Port = 8080
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$repoRootPath = $repoRoot.Path
$escapedRepoRoot = [regex]::Escape($repoRootPath)
$script:StoppedProcessIds = @{}
$script:StoppedProcessCount = 0

Set-Location $repoRoot

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
    $script:StoppedProcessCount += 1

    Write-Host "Stopping $($process.Name) process $ProcessId ($Reason)."
    Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
    Wait-Process -Id $ProcessId -Timeout 5 -ErrorAction SilentlyContinue
}

$allProcesses = Get-CimInstance Win32_Process -ErrorAction SilentlyContinue

$dioxusServers = $allProcesses |
    Where-Object {
        $_.Name -ieq "dx.exe" `
            -and $_.CommandLine -match "\bserve\b" `
            -and $_.CommandLine -match "--port\s+$Port(\s|$)"
    }

$webPortListeners = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue |
    Select-Object -ExpandProperty OwningProcess -Unique

$gameServers = $allProcesses |
    Where-Object {
        ($_.Name -like "server-*.exe" -or $_.Name -ieq "web.exe") `
            -and $_.CommandLine -match $escapedRepoRoot `
            -and $_.CommandLine -match "\\target\\dx\\web\\"
    }

$dioxusServers |
    ForEach-Object {
        Stop-ProcessById -ProcessId $_.ProcessId -Reason "Dioxus web server on port $Port"
    }

$webPortListeners |
    ForEach-Object {
        Stop-ProcessById -ProcessId $_ -Reason "web port $Port listener"
    }

$gameServers |
    ForEach-Object {
        Stop-ProcessById -ProcessId $_.ProcessId -Reason "generated game server for this repository"
    }

if ($script:StoppedProcessCount -eq 0) {
    Write-Host "No Dioxus web or generated game server processes were found for port $Port."
} else {
    Write-Host "Stopped $script:StoppedProcessCount web/game server process(es)."
}
