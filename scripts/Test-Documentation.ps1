[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$pairs = @(
    @('README.md', 'README.zh-TW.md'),
    @('THIRD_PARTY_NOTICES.md', 'THIRD_PARTY_NOTICES.zh-TW.md'),
    @('docs/user/guide.md', 'docs/user/guide.zh-TW.md'),
    @('docs/user/requirements.md', 'docs/user/requirements.zh-TW.md'),
    @('docs/user/sharing.md', 'docs/user/sharing.zh-TW.md'),
    @('docs/release/release-notes.md', 'docs/release/release-notes.zh-TW.md'),
    @('docs/release/known-issues.md', 'docs/release/known-issues.zh-TW.md'),
    @('docs/release/release-candidate-runbook.md', 'docs/release/release-candidate-runbook.zh-TW.md'),
    @('docs/release/windows-installer.md', 'docs/release/windows-installer.zh-TW.md'),
    @('docs/release/updates-and-recovery.md', 'docs/release/updates-and-recovery.zh-TW.md'),
    @('docs/testing/stage-9-gpu-support-matrix.md', 'docs/testing/stage-9-gpu-support-matrix.zh-TW.md'),
    @('docs/legal/licensing-and-distribution.md', 'docs/legal/licensing-and-distribution.zh-TW.md'),
    @('docs/development/README.md', 'docs/development/README.zh-TW.md'),
    @('docs/development/roadmap.md', 'docs/development/roadmap.zh-TW.md'),
    @('docs/ui/README.md', 'docs/ui/README.zh-TW.md')
)

foreach ($pair in $pairs) {
    $englishPath = Join-Path $repoRoot $pair[0]
    $chinesePath = Join-Path $repoRoot $pair[1]
    if (-not (Test-Path -LiteralPath $englishPath -PathType Leaf) -or
        -not (Test-Path -LiteralPath $chinesePath -PathType Leaf)) {
        throw "Documentation translation pair is missing: $($pair -join ', ')"
    }
    $english = Get-Content -LiteralPath $englishPath -Raw -Encoding UTF8
    $chinese = Get-Content -LiteralPath $chinesePath -Raw -Encoding UTF8
    if (-not $english.Contains((Split-Path -Leaf $chinesePath)) -or
        -not $chinese.Contains((Split-Path -Leaf $englishPath))) {
        throw "Documentation language switch is missing: $($pair -join ', ')"
    }
    foreach ($path in @($englishPath, $chinesePath)) {
        $source = Get-Content -LiteralPath $path -Raw -Encoding UTF8
        foreach ($match in [regex]::Matches($source, '!?\[[^\]]+\]\(([^)]+)\)')) {
            $target = $match.Groups[1].Value.Split('#')[0]
            if ([string]::IsNullOrWhiteSpace($target) -or $target -match '^[a-z][a-z0-9+.-]*://' -or $target.StartsWith('mailto:')) {
                continue
            }
            $resolved = [System.IO.Path]::GetFullPath((Join-Path (Split-Path -Parent $path) $target))
            if (-not (Test-Path -LiteralPath $resolved)) {
                throw "Broken documentation link in $path : $target"
            }
        }
    }
}

$readme = Get-Content -LiteralPath (Join-Path $repoRoot 'README.md') -Raw -Encoding UTF8
if ($readme -match '(?m)^##\s*(?:\u958B\u767C\u6B65\u9A5F|\u7CFB\u7D71\u67B6\u69CB)\s*$') {
    throw 'The root README must remain user-facing; move developer plans to docs/development.'
}

Write-Host "Documentation language pairs and local links passed: $($pairs.Count) pairs." -ForegroundColor Green
