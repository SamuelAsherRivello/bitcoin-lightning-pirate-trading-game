param(
    [string]$Bump
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..\..\..\..")
$cachePath = Join-Path $repoRoot ".cache\list-skill-repetoir\skill-counts.tsv"

if (-not (Test-Path -LiteralPath $cachePath)) {
    throw "Missing skill count cache: $cachePath"
}

$rows = Import-Csv -LiteralPath $cachePath -Delimiter "`t"

if ($Bump) {
    $matched = $false
    foreach ($row in $rows) {
        if ($row.skill -eq $Bump) {
            $count = [int]$row.call_count
            $row.call_count = "{0:D3}" -f ($count + 1)
            $matched = $true
            break
        }
    }

    if (-not $matched) {
        throw "Skill '$Bump' was not found in $cachePath"
    }

    $lines = @("skill`tsummary`tcall_count")
    foreach ($row in $rows) {
        $lines += "$($row.skill)`t$($row.summary)`t$($row.call_count)"
    }
    Set-Content -LiteralPath $cachePath -Value $lines -Encoding utf8
}

function ConvertTo-MarkdownCell {
    param([string]$Value)

    return ($Value -replace '\|', '\|').Trim()
}

"| Skill | Summary | CallCount |"
"| --- | --- | --- |"
foreach ($row in $rows) {
    $skill = ConvertTo-MarkdownCell $row.skill
    $summary = ConvertTo-MarkdownCell $row.summary
    $callCount = ConvertTo-MarkdownCell $row.call_count
    "| $skill | $summary | $callCount |"
}
