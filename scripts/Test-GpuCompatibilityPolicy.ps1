[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$matrixPath = Join-Path $repoRoot 'docs\testing\gpu-support-matrix.json'
$policyPath = Join-Path $repoRoot 'app\src-tauri\src\gpu_compatibility.rs'
$readmePath = Join-Path $repoRoot 'README.md'

foreach ($path in @($matrixPath, $policyPath, $readmePath)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required GPU compatibility file is missing: $path"
    }
}

$matrix = Get-Content -LiteralPath $matrixPath -Raw | ConvertFrom-Json
if ($matrix.schemaVersion -ne 1) {
    throw 'The GPU support matrix schema version must be 1.'
}
if ($matrix.policy.minimumDriver -ne '560.76' -or
    $matrix.policy.minimumComputeCapability -ne '7.5' -or
    $matrix.policy.bf16ComputeCapability -ne '8.0' -or
    $matrix.policy.minimumVramMb -ne 2048 -or
    $matrix.policy.recommendedVramMb -ne 8192 -or
    $matrix.policy.minimumFreeVramMb -ne 1536) {
    throw 'The machine-readable GPU policy does not match the reviewed Stage 9.7 thresholds.'
}

$tiers = @($matrix.physicalRecords | ForEach-Object { $_.tier })
if ('recommended' -notin $tiers -or 'minimum_low_vram' -notin $tiers) {
    throw 'Physical records must include recommended and minimum/low-VRAM evidence.'
}
foreach ($record in $matrix.physicalRecords) {
    if ($record.install -ne 'pass' -or $record.modelLoad -ne 'pass' -or $record.peakVramMb -le 0) {
        throw "Physical GPU record is incomplete: $($record.gpu)"
    }
}

$policySource = Get-Content -LiteralPath $policyPath -Raw
$requiredConstants = @(
    'MINIMUM_DRIVER: (u32, u32) = (560, 76)',
    'MINIMUM_VRAM_MB: u64 = 2_048',
    'RECOMMENDED_VRAM_MB: u64 = 8_192',
    'MINIMUM_FREE_VRAM_MB: u64 = 1_536',
    'MINIMUM_COMPUTE_CAPABILITY: (u32, u32) = (7, 5)',
    'BF16_COMPUTE_CAPABILITY: (u32, u32) = (8, 0)'
)
foreach ($constant in $requiredConstants) {
    if (-not $policySource.Contains($constant)) {
        throw "Rust GPU policy is missing the reviewed threshold: $constant"
    }
}

$readme = Get-Content -LiteralPath $readmePath -Raw
if ($readme -notmatch '(?m)^- \[x\] 9\.7') {
    throw 'README must mark Stage 9.7 complete only when this policy gate is present.'
}

Write-Host 'GPU compatibility policy and physical evidence matrix passed.' -ForegroundColor Green
