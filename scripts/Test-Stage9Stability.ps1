[CmdletBinding()]
param(
    [ValidateRange(1, 1000)]
    [int]$Iterations = 50
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$rustRoot = Join-Path $repoRoot 'app\src-tauri'
$originalIterations = $env:BEATFORGE_SOAK_ITERATIONS
$originalDelay = $env:BEATFORGE_MOCK_STAGE_DELAY_MS
$startedAt = Get-Date
$processesBefore = @(Get-Process -Name 'app' -ErrorAction SilentlyContinue).Count

if (-not (Get-Command 'cargo.exe' -ErrorAction SilentlyContinue)) {
    throw 'Required development tool is not available: cargo.exe'
}

try {
    $env:BEATFORGE_SOAK_ITERATIONS = $Iterations.ToString([Globalization.CultureInfo]::InvariantCulture)
    $env:BEATFORGE_MOCK_STAGE_DELAY_MS = '5'
    Push-Location $rustRoot
    try {
        & cargo.exe test --locked --test mock_worker_process sequential_worker_soak_keeps_jobs_isolated -- --exact --nocapture
        if ($LASTEXITCODE -ne 0) {
            throw "Stage 9 worker soak failed with exit code $LASTEXITCODE"
        }
    }
    finally {
        Pop-Location
    }

    $processesAfter = @(Get-Process -Name 'app' -ErrorAction SilentlyContinue).Count
    if ($processesAfter -ne $processesBefore) {
        throw "Worker process count changed from $processesBefore to $processesAfter"
    }
    $seconds = [Math]::Round(((Get-Date) - $startedAt).TotalSeconds, 1)
    Write-Host "Stage 9 worker soak passed: $Iterations isolated jobs in $seconds seconds; no worker process remained." -ForegroundColor Green
}
finally {
    if ($null -eq $originalIterations) {
        Remove-Item Env:BEATFORGE_SOAK_ITERATIONS -ErrorAction SilentlyContinue
    }
    else {
        $env:BEATFORGE_SOAK_ITERATIONS = $originalIterations
    }
    if ($null -eq $originalDelay) {
        Remove-Item Env:BEATFORGE_MOCK_STAGE_DELAY_MS -ErrorAction SilentlyContinue
    }
    else {
        $env:BEATFORGE_MOCK_STAGE_DELAY_MS = $originalDelay
    }
}
