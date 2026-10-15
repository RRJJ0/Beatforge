[CmdletBinding()]
param(
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$appRoot = Join-Path $repoRoot 'app'
$rustRoot = Join-Path $appRoot 'src-tauri'
$runRootParent = Join-Path $appRoot '.release-test-runs'
$runName = 'run-' + [DateTimeOffset]::UtcNow.ToString('yyyyMMdd-HHmmss-fff') + '-' + $PID
$runRoot = Join-Path $runRootParent $runName
$originalTemp = $env:TEMP
$originalTmp = $env:TMP
$originalTestRoot = $env:BEATFORGE_TEST_ROOT
$startedAt = Get-Date
$stepResults = [System.Collections.Generic.List[object]]::new()

function Assert-Tool {
    param([Parameter(Mandatory)][string]$Name)

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Required development tool is not available: $Name"
    }
}

function Invoke-ReleaseStep {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][scriptblock]$Action
    )

    Write-Host "`n==> $Name" -ForegroundColor Cyan
    $stepStartedAt = Get-Date
    $global:LASTEXITCODE = 0
    & $Action
    $exitCode = $global:LASTEXITCODE
    if ($exitCode -ne 0) {
        throw "$Name failed with exit code $exitCode"
    }
    $duration = [Math]::Round(((Get-Date) - $stepStartedAt).TotalSeconds, 1)
    $stepResults.Add([pscustomobject]@{ Step = $Name; Seconds = $duration })
}

Assert-Tool 'npm.cmd'
Assert-Tool 'node.exe'
Assert-Tool 'cargo.exe'
Assert-Tool 'rustc.exe'

$pythonCommand = Get-Command 'python.exe' -ErrorAction SilentlyContinue
if (-not $pythonCommand) {
    throw 'Required development tool is not available: python.exe'
}

Write-Host 'Release baseline toolchain:' -ForegroundColor Cyan
& node.exe --version
& npm.cmd --version
& rustc.exe --version
& cargo.exe --version
& $pythonCommand.Source --version

if (-not (Test-Path -LiteralPath (Join-Path $appRoot 'node_modules') -PathType Container)) {
    throw 'Frontend dependencies are missing. Run pnpm install --frozen-lockfile in app first.'
}
if (-not (Test-Path -LiteralPath (Join-Path $appRoot 'pnpm-lock.yaml') -PathType Leaf)) {
    throw 'The pnpm lockfile is required for release verification.'
}
if (-not (Test-Path -LiteralPath (Join-Path $rustRoot 'Cargo.lock') -PathType Leaf)) {
    throw 'The Cargo lockfile is required for release verification.'
}

New-Item -ItemType Directory -Path $runRoot -Force | Out-Null
$isolatedTemp = Join-Path $runRoot 'temp'
New-Item -ItemType Directory -Path $isolatedTemp -Force | Out-Null
$env:TEMP = $isolatedTemp
$env:TMP = $isolatedTemp
$env:BEATFORGE_TEST_ROOT = $runRoot

try {
    Invoke-ReleaseStep 'Release fixture policy' {
        & (Join-Path $PSScriptRoot 'Test-ReleaseFixtures.ps1')
    }
    Invoke-ReleaseStep 'Repository privacy policy' {
        & (Join-Path $PSScriptRoot 'Test-RepositoryPrivacy.ps1')
    }

    Push-Location $appRoot
    try {
        Invoke-ReleaseStep 'Frontend type-check' { & npm.cmd run type-check }
        Invoke-ReleaseStep 'Frontend lint (read-only)' { & npm.cmd run lint:check }
        Invoke-ReleaseStep 'Frontend unit tests' { & npm.cmd run test:unit:run }
        if (-not $SkipBuild) {
            Invoke-ReleaseStep 'Frontend production build' { & npm.cmd run build-only }
        }
    }
    finally {
        Pop-Location
    }

    Push-Location $rustRoot
    try {
        Invoke-ReleaseStep 'Rust formatting' { & cargo.exe fmt --all -- --check }
        Invoke-ReleaseStep 'Rust lint' { & cargo.exe clippy --locked --all-targets --all-features -- -D warnings }
        Invoke-ReleaseStep 'Rust tests' { & cargo.exe test --locked --all-features }
    }
    finally {
        Pop-Location
    }

    Push-Location (Join-Path $rustRoot 'resources\python')
    try {
        Invoke-ReleaseStep 'Python worker tests' {
            & $pythonCommand.Source -m unittest discover -s . -p 'test_*.py' -v
        }
    }
    finally {
        Pop-Location
    }

    Write-Host "`nRelease baseline passed in $([Math]::Round(((Get-Date) - $startedAt).TotalSeconds, 1)) seconds." -ForegroundColor Green
    $stepResults | Format-Table -AutoSize
}
finally {
    $env:TEMP = $originalTemp
    $env:TMP = $originalTmp
    if ($null -eq $originalTestRoot) {
        Remove-Item Env:BEATFORGE_TEST_ROOT -ErrorAction SilentlyContinue
    }
    else {
        $env:BEATFORGE_TEST_ROOT = $originalTestRoot
    }

    $resolvedParent = [System.IO.Path]::GetFullPath($runRootParent).TrimEnd('\') + '\'
    $resolvedRunRoot = [System.IO.Path]::GetFullPath($runRoot)
    if (
        $resolvedRunRoot.StartsWith($resolvedParent, [System.StringComparison]::OrdinalIgnoreCase) -and
        (Split-Path -Leaf $resolvedRunRoot) -match '^run-\d{8}-\d{6}-\d{3}-\d+$' -and
        (Test-Path -LiteralPath $resolvedRunRoot)
    ) {
        Remove-Item -LiteralPath $resolvedRunRoot -Recurse -Force
    }
}
