$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$runWebPath = Join-Path $repoRoot "Scripts\Common\RunWeb.ps1"
$installDependenciesPath = Join-Path $repoRoot "Scripts\Common\InstallDependencies.ps1"
$runWeb = Get-Content -Raw -Path $runWebPath
$installDependencies = Get-Content -Raw -Path $installDependenciesPath

function Assert-Contains {
    param(
        [string]$Content,
        [string]$Pattern,
        [string]$Message
    )

    if ($Content -notmatch $Pattern) {
        throw $Message
    }
}

Assert-Contains `
    -Content $runWeb `
    -Pattern '\[switch\]\s*\$NoOpen' `
    -Message "RunWeb.ps1 should expose -NoOpen so browser launch can be disabled for automation."

Assert-Contains `
    -Content $runWeb `
    -Pattern 'Start-Process\s+-FilePath\s+\$(appUrl|Url)' `
    -Message "RunWeb.ps1 should open the served app URL in the default browser."

Assert-Contains `
    -Content $runWeb `
    -Pattern 'if\s*\(\s*-not\s+\$NoOpen\s*\)' `
    -Message "RunWeb.ps1 should only open the browser when -NoOpen is not set."

Assert-Contains `
    -Content $installDependencies `
    -Pattern 'function\s+Get-WorkspaceDioxusVersion' `
    -Message "InstallDependencies.ps1 should derive the required Dioxus CLI version from the workspace manifest."

Assert-Contains `
    -Content $installDependencies `
    -Pattern '\$RequiredDxVersion\s*=\s*Get-WorkspaceDioxusVersion' `
    -Message "InstallDependencies.ps1 should use the workspace Dioxus version as the required dx version."

Assert-Contains `
    -Content $installDependencies `
    -Pattern '(?m)^\s*if\s*\(\$DxVersionOutput\s+-match\s+\$ExpectedDxVersionPattern\)' `
    -Message "InstallDependencies.ps1 should compare dx against the exact required Dioxus version."

Assert-Contains `
    -Content $installDependencies `
    -Pattern 'function\s+Invoke-CheckedCommand' `
    -Message "InstallDependencies.ps1 should fail when native commands fail."

Assert-Contains `
    -Content $installDependencies `
    -Pattern '"dioxus-cli@\$RequiredDxVersion",\s*"--locked",\s*"--force"' `
    -Message "InstallDependencies.ps1 should install the matching Dioxus CLI with locked dependencies."

Write-Host "Script tests passed."
