param(
    [string]$Address = "",
    [int]$Port = 8080,
    [int]$AuthBridgePort = 37374,
    [switch]$NoOpen,
    [switch]$Restart
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$repoRootPath = $repoRoot.Path
$escapedRepoRoot = [regex]::Escape($repoRootPath)
$script:StoppedProcessIds = @{}
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
    Write-Host "Stopping $($process.Name) process $ProcessId ($Reason)."
    Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
    Wait-Process -Id $ProcessId -Timeout 5 -ErrorAction SilentlyContinue
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

$authBridgeProcesses = $allProcesses |
    Where-Object {
        ($_.Name -ieq "lnauth-bridge.exe" -or ($_.Name -ieq "cargo.exe" -and $_.CommandLine -match "\brun\b" -and $_.CommandLine -match "\blnauth-bridge\b")) `
            -and $_.CommandLine -match $escapedRepoRoot
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

$dioxusServers |
    ForEach-Object {
        Stop-ProcessById -ProcessId $_.ProcessId -Reason "Dioxus static/hot-reload server on port $Port"
    }

$gameServers |
    ForEach-Object {
        Stop-ProcessById -ProcessId $_.ProcessId -Reason "generated app server for this repository"
    }

$authBridgeProcesses |
    ForEach-Object {
        Stop-ProcessById -ProcessId $_.ProcessId -Reason "LNAuth bridge for this repository"
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

$wifiAddress = Get-NetIPConfiguration |
    Where-Object { $_.IPv4DefaultGateway -and $_.NetAdapter.Status -eq "Up" } |
    Select-Object -ExpandProperty IPv4Address -First 1 |
    Select-Object -ExpandProperty IPAddress

$bindAddress = $Address.Trim()
$browserHost = $bindAddress

if ([string]::IsNullOrWhiteSpace($bindAddress)) {
    $bindAddress = "127.0.0.1"
    $browserHost = "localhost"
} elseif ($bindAddress -ieq "localhost") {
    $bindAddress = "127.0.0.1"
    $browserHost = "localhost"
} elseif ($bindAddress -eq "127.0.0.1") {
    $browserHost = "localhost"
} elseif ($bindAddress -eq "0.0.0.0") {
    if ($wifiAddress) {
        Write-Host "Fullstack backend readiness can fail on Windows when using 0.0.0.0. Using Wi-Fi address $wifiAddress instead."
        $bindAddress = $wifiAddress
        $browserHost = $wifiAddress
    } else {
        Write-Host "Fullstack backend readiness can fail on Windows when using 0.0.0.0. Using 127.0.0.1 instead."
        $bindAddress = "127.0.0.1"
        $browserHost = "localhost"
    }
}

$appUrl = "http://$browserHost`:$Port"

Write-Host "Starting web app."
Write-Host "Laptop: $appUrl"
if ($browserHost -eq "localhost") {
    Write-Host "Polar:  use this localhost URL when testing Polar Automation with http://localhost:37373."
    Write-Host "LNAuth: phone scanning needs a LAN address. Restart with -Address <this laptop's Wi-Fi IPv4>."
} elseif ($bindAddress -eq $wifiAddress) {
    Write-Host "Phone:  http://$bindAddress`:$Port"
    Write-Host "LNAuth: phone wallet callbacks use http://$bindAddress`:$AuthBridgePort."
    Write-Host "Polar:  browser calls to http://localhost:37373 may fail from non-localhost origins. Use -Address 127.0.0.1 for Polar Automation."
} else {
    Write-Host "Phone:  not available unless you pass this laptop's Wi-Fi IPv4 address with -Address."
    Write-Host "LNAuth: phone wallet callbacks use http://$bindAddress`:$AuthBridgePort if your phone can reach that address."
}
Write-Host ""

if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
    throw "npm is required for Tailwind CSS. Run Scripts\Common\InstallDependencies.ps1 first."
}

Write-Host "Building Tailwind CSS..."
npm run tailwind:build

$env:LNAUTH_BRIDGE_ADDRESS = $bindAddress
$env:LNAUTH_BRIDGE_PORT = "$AuthBridgePort"
Write-Host "Starting LNAuth bridge at http://$bindAddress`:$AuthBridgePort."
Write-Host "Building LNAuth bridge..."
cargo build -p lnauth-bridge
$authBridgeExe = Join-Path $repoRootPath "target\debug\lnauth-bridge.exe"
Start-Process -FilePath $authBridgeExe `
    -WorkingDirectory $repoRootPath `
    -WindowStyle Hidden | Out-Null

if (-not $NoOpen) {
    Write-Host "Browser: will open $appUrl when the web server is ready."
    Start-Job -Name "OpenDioxusWeb-$Port" -ScriptBlock {
        param(
            [string]$Url
        )

        for ($attempt = 0; $attempt -lt 120; $attempt += 1) {
            try {
                Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 2 | Out-Null
                Start-Process -FilePath $Url
                return
            } catch {
                Start-Sleep -Milliseconds 500
            }
        }

        Start-Process -FilePath $Url
    } -ArgumentList $appUrl | Out-Null
} else {
    Write-Host "Browser: -NoOpen was set; open $appUrl manually when needed."
}

dx serve --platform web --addr $bindAddress --port $Port
