[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if (-not (Get-Command 'git.exe' -ErrorAction SilentlyContinue)) {
    throw 'Required development tool is not available: git.exe'
}

Push-Location $repoRoot
try {
    $revisions = @(git.exe rev-list --all)
    if ($LASTEXITCODE -ne 0 -or $revisions.Count -eq 0) {
        throw 'Could not enumerate Git history'
    }

    $policyPath = 'scripts/Test-RepositoryPrivacy.ps1'
    $patterns = @(
        @{
            Name = 'Windows user path'
            Regex = 'C:\\Users\\'
            AllowedPaths = @(
                $policyPath,
                'README.md',
                'docs/development/roadmap.zh-TW.md',
                'app/src/views/ModelsView.vue',
                'app/src-tauri/src/diagnostics.rs',
                'docs/testing/stage-9-privacy-scan.md'
            )
        },
        @{ Name = 'Known developer username'; Regex = ('rj' + '199'); AllowedPaths = @($policyPath) },
        @{ Name = 'AWS access key'; Regex = 'AKIA[0-9A-Z]{16}'; AllowedPaths = @($policyPath) },
        @{ Name = 'Private key'; Regex = 'BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY'; AllowedPaths = @($policyPath) },
        @{ Name = 'GitHub token'; Regex = '(gh[pousr]_[A-Za-z0-9_]{20,}|github_pat_[A-Za-z0-9_]{20,})'; AllowedPaths = @($policyPath) },
        @{ Name = 'OpenAI-style secret'; Regex = 'sk-[A-Za-z0-9]{20,}'; AllowedPaths = @($policyPath) }
    )
    $violations = [System.Collections.Generic.List[string]]::new()
    foreach ($rule in $patterns) {
        $paths = @(
            git.exe log --all --format= --name-only -G $rule.Regex -- . `
                | Where-Object { $_ } `
                | Sort-Object -Unique
        )
        foreach ($path in $paths) {
            if ($rule.AllowedPaths -contains $path) {
                continue
            }
            $violations.Add("$($rule.Name): $path")
        }
    }

    $mediaPattern = '\.(mp3|wav|ogg|m4a|flac|osu|osz|png|jpe?g|gif|mp4|avi|flv|db|sqlite|safetensors|pt|pth|ckpt)$'
    $objects = @(git.exe rev-list --objects --all)
    foreach ($line in $objects) {
        $space = $line.IndexOf(' ')
        if ($space -lt 0) { continue }
        $path = $line.Substring($space + 1).Replace('\', '/')
        if ($path -notmatch $mediaPattern) { continue }
        $allowedIcon = $path -match '^app/src-tauri/icons/[^/]+\.png$'
        $allowedFixture = $path -match '^app/src-tauri/tests/fixtures/validation/(valid|invalid|warnings)/[^/]+\.osu$'
        if (-not $allowedIcon -and -not $allowedFixture) {
            $violations.Add("Prohibited media/data history object: $path")
        }
    }

    if ($violations.Count -gt 0) {
        $summary = $violations | Sort-Object -Unique
        throw "Repository privacy scan found $($summary.Count) issue(s):`n$($summary -join "`n")"
    }
    Write-Host "Repository privacy scan passed: $($revisions.Count) revisions; no credential, personal-path, song, generated beatmap, model, or database history objects." -ForegroundColor Green
}
finally {
    Pop-Location
}
