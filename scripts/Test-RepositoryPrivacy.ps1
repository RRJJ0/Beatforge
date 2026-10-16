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

    # Only these manually reviewed, redacted documentation screenshots are public.
    # Pin their Git blob IDs so replacing a screenshot requires another review.
    $allowedScreenshots = @{
        'docs/demoImage/generate.png' = '22799c6cfd713afc7274fee2e539ffc7661d9342'
        'docs/demoImage/history.png' = '66dc851bd865ed4850cbd238fbd2e940480cd684'
        'docs/demoImage/jobs.png' = 'f8cea9d381d8a753699b29a96c8c292bd0e74f05'
        'docs/demoImage/model.png' = 'c60d85b9210c5444356e201057b190cb07c72fbc'
        'docs/demoImage/setting-1.png' = '74fad5cab422db4782ff50b94f3b79af045a40bd'
        'docs/demoImage/setting-2.png' = 'f77fce4c75e47db02a8b50b8d8937b19be4c7242'
    }
    $mediaPattern = '\.(mp3|wav|ogg|m4a|flac|osu|osz|png|jpe?g|gif|mp4|avi|flv|db|sqlite|safetensors|pt|pth|ckpt)$'
    $objects = @(git.exe rev-list --objects --all)
    foreach ($line in $objects) {
        $space = $line.IndexOf(' ')
        if ($space -lt 0) { continue }
        $path = $line.Substring($space + 1).Replace('\', '/')
        if ($path -notmatch $mediaPattern) { continue }
        $objectId = $line.Substring(0, $space)
        $allowedIcon = $path -match '^app/src-tauri/icons/[^/]+\.png$'
        $allowedFixture = $path -match '^app/src-tauri/tests/fixtures/validation/(valid|invalid|warnings)/[^/]+\.osu$'
        $allowedScreenshot = $allowedScreenshots.ContainsKey($path) -and $allowedScreenshots[$path] -eq $objectId
        if (-not $allowedIcon -and -not $allowedFixture -and -not $allowedScreenshot) {
            $violations.Add("Prohibited media/data history object: $path")
        }
    }

    if ($violations.Count -gt 0) {
        $summary = $violations | Sort-Object -Unique
        throw "Repository privacy scan found $($summary.Count) issue(s):`n$($summary -join "`n")"
    }
    Write-Host "Repository privacy scan passed: $($revisions.Count) revisions; only reviewed documentation screenshots are allowlisted media." -ForegroundColor Green
}
finally {
    Pop-Location
}
