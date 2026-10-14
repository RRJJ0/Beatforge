[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$manifestPath = Join-Path $repoRoot 'tests\fixtures\release\manifest.json'
$strictUtf8 = [System.Text.UTF8Encoding]::new($false, $true)

if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw "Release fixture manifest is missing: $manifestPath"
}

$manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($manifest.schemaVersion -ne 1) {
    throw "Unsupported release fixture manifest version: $($manifest.schemaVersion)"
}

$listedPaths = [System.Collections.Generic.HashSet[string]]::new(
    [System.StringComparer]::OrdinalIgnoreCase
)
$allowedExtensions = [System.Collections.Generic.HashSet[string]]::new(
    [System.StringComparer]::OrdinalIgnoreCase
)
foreach ($extension in $manifest.allowedExtensions) {
    [void]$allowedExtensions.Add([string]$extension)
}

$repoPrefix = $repoRoot.TrimEnd('\') + '\'
foreach ($fixture in $manifest.committedFixtures) {
    $relativePath = ([string]$fixture.path).Replace('/', '\')
    $absolutePath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $relativePath))
    if (-not $absolutePath.StartsWith($repoPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Fixture escapes the repository root: $relativePath"
    }
    if (-not $listedPaths.Add($absolutePath)) {
        throw "Fixture is listed more than once: $relativePath"
    }
    if (-not (Test-Path -LiteralPath $absolutePath -PathType Leaf)) {
        throw "Listed fixture does not exist: $relativePath"
    }

    $file = Get-Item -LiteralPath $absolutePath
    if ($file.Length -gt [int64]$manifest.maxCommittedFixtureBytes) {
        throw "Fixture exceeds $($manifest.maxCommittedFixtureBytes) bytes: $relativePath"
    }
    if (-not $allowedExtensions.Contains($file.Extension)) {
        throw "Fixture extension is not allowed: $relativePath"
    }

    try {
        $text = $strictUtf8.GetString([System.IO.File]::ReadAllBytes($absolutePath))
    }
    catch {
        throw "Fixture is not valid UTF-8: $relativePath"
    }
    if ($text -match '(?i)(?:[A-Z]:\\Users\\|/Users/|/home/)[^\s\"'']+') {
        throw "Fixture appears to contain a personal absolute path: $relativePath"
    }
}

$discoveredPaths = [System.Collections.Generic.HashSet[string]]::new(
    [System.StringComparer]::OrdinalIgnoreCase
)
foreach ($fixtureRoot in $manifest.fixtureRoots) {
    $relativeRoot = ([string]$fixtureRoot).Replace('/', '\')
    $absoluteRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $relativeRoot))
    if (-not $absoluteRoot.StartsWith($repoPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Fixture root escapes the repository: $relativeRoot"
    }
    if (-not (Test-Path -LiteralPath $absoluteRoot -PathType Container)) {
        throw "Fixture root does not exist: $relativeRoot"
    }
    foreach ($file in Get-ChildItem -LiteralPath $absoluteRoot -File -Recurse) {
        [void]$discoveredPaths.Add($file.FullName)
    }
}

$unlisted = @($discoveredPaths | Where-Object { -not $listedPaths.Contains($_) })
$missing = @($listedPaths | Where-Object { -not $discoveredPaths.Contains($_) })
if ($unlisted.Count -gt 0 -or $missing.Count -gt 0) {
    $details = @()
    if ($unlisted.Count -gt 0) {
        $details += "Unlisted fixtures:`n  $($unlisted -join "`n  ")"
    }
    if ($missing.Count -gt 0) {
        $details += "Manifest entries outside fixture roots:`n  $($missing -join "`n  ")"
    }
    throw ($details -join "`n")
}

Write-Host "Release fixtures verified: $($listedPaths.Count) committed, $($manifest.generatedFixtures.Count) generated categories."
