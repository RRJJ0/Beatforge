[CmdletBinding()]
param(
    [switch]$AllowUnsignedLocalSmoke,
    [switch]$AllowUnsignedPublicRelease,
    [switch]$TimestampUsesRfc3161,
    [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if ($AllowUnsignedLocalSmoke -and $AllowUnsignedPublicRelease) {
    throw 'Choose either -AllowUnsignedLocalSmoke or -AllowUnsignedPublicRelease.'
}
$status = @(& git.exe -C $repoRoot status --porcelain)
if ($LASTEXITCODE -ne 0) { throw 'Unable to read Git source status' }
if ($status.Count -gt 0 -and -not $AllowUnsignedLocalSmoke) {
    throw 'A publishable release candidate requires a clean committed source revision.'
}

& (Join-Path $PSScriptRoot 'Test-Release.ps1')
if ($LASTEXITCODE -ne 0) { throw 'GPU-free release gate failed' }

$buildArgs = @{ SkipTests = $true }
if ($AllowUnsignedLocalSmoke) {
    $buildArgs.AllowUnsigned = $true
    $buildArgs.AllowDirty = $true
}
if ($AllowUnsignedPublicRelease) {
    $buildArgs.AllowUnsigned = $true
    $buildArgs.AllowUnsignedPublicRelease = $true
}
if ($TimestampUsesRfc3161) { $buildArgs.TimestampUsesRfc3161 = $true }
if (-not [string]::IsNullOrWhiteSpace($OutputDirectory)) { $buildArgs.OutputDirectory = $OutputDirectory }
& (Join-Path $PSScriptRoot 'Build-WindowsInstaller.ps1') @buildArgs
if ($LASTEXITCODE -ne 0) { throw 'Windows installer build failed' }

if ($AllowUnsignedLocalSmoke) {
    Write-Warning 'Local smoke artifact only. Do not publish: Authenticode and clean-machine acceptance are not complete.'
} elseif ($AllowUnsignedPublicRelease) {
    Write-Warning 'Unsigned public candidate built. Windows has no verified Publisher; disclose this clearly and complete clean-machine/update acceptance before publication.'
} else {
    Write-Host 'Signed artifact built. Human clean-machine, update and Publisher/SmartScreen acceptance remain mandatory.' -ForegroundColor Yellow
}
