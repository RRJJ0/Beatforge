[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$inventoryPath = Join-Path $repoRoot 'third_party\dependency-inventory.json'
$testRoot = Join-Path $repoRoot 'app\.release-test-runs'
$candidatePath = Join-Path $testRoot "license-inventory-$PID.json"

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

foreach ($required in @(
    'LICENSE',
    'THIRD_PARTY_NOTICES.md',
    'THIRD_PARTY_NOTICES.zh-TW.md',
    'docs\legal\licensing-and-distribution.md',
    'third_party\component-policy.json',
    'third_party\python-licenses.json',
    'third_party\dependency-inventory.json'
)) {
    Assert-True (Test-Path -LiteralPath (Join-Path $repoRoot $required) -PathType Leaf) "Missing legal release file: $required"
}

$cargoToml = Get-Content -LiteralPath (Join-Path $repoRoot 'app\src-tauri\Cargo.toml') -Raw
Assert-True ($cargoToml -match '(?m)^license = "MIT"$') 'Rust package metadata must declare MIT'
Assert-True ($cargoToml -match '(?m)^repository = "https://github.com/RRJJ0/Beatforge"\r?$') 'Rust package repository is missing or unexpected'

$bundle = Get-Content -LiteralPath (Join-Path $repoRoot 'app\src-tauri\tauri.conf.json') -Raw | ConvertFrom-Json
$resources = $bundle.bundle.resources
Assert-True ($resources.'../../LICENSE' -eq 'legal/LICENSE') 'Installer does not include Beatforge LICENSE'
Assert-True ($resources.'../../THIRD_PARTY_NOTICES.md' -eq 'legal/THIRD_PARTY_NOTICES.md') 'Installer does not include third-party notices'
Assert-True ($resources.'../../THIRD_PARTY_NOTICES.zh-TW.md' -eq 'legal/THIRD_PARTY_NOTICES.zh-TW.md') 'Installer does not include Traditional Chinese third-party notices'
Assert-True ($resources.'../../third_party/dependency-inventory.json' -eq 'legal/dependency-inventory.json') 'Installer does not include dependency inventory'

$resourceSources = @($resources.PSObject.Properties.Name)
$forbiddenResource = $resourceSources | Where-Object { $_ -match '(?i)(\.pt$|\.safetensors$|\.onnx$|python|ffmpeg|mapperatorinator|model|\.osu$|\.osz$|\.(mp3|wav|ogg|flac|mp4|avi)$)' }
Assert-True ($null -eq $forbiddenResource) "Download-only or user content appears in installer resources: $forbiddenResource"

$mapperLock = Get-Content -LiteralPath (Join-Path $repoRoot 'app\src-tauri\resources\manifests\mapperatorinator-v32.lock.json') -Raw | ConvertFrom-Json
$componentPolicy = Get-Content -LiteralPath (Join-Path $repoRoot 'third_party\component-policy.json') -Raw | ConvertFrom-Json
$components = @{}
foreach ($component in $componentPolicy.components) { $components[$component.name] = $component }
Assert-True ($components['Mapperatorinator'].version -eq $mapperLock.source.commit) 'Mapperatorinator license policy pin differs from runtime lock'
Assert-True ($components['slider'].version -eq $mapperLock.slider.commit) 'slider license policy pin differs from runtime lock'
Assert-True ($components['Mapperatorinator-v32 model'].version -eq $mapperLock.model.revision) 'Model license policy pin differs from runtime lock'
foreach ($component in $componentPolicy.components) {
    Assert-True ($component.distribution -match '^download_only') "Managed component is not explicitly download-only: $($component.name)"
}

New-Item -ItemType Directory -Path $testRoot -Force | Out-Null
try {
    & (Join-Path $PSScriptRoot 'Update-ThirdPartyInventory.ps1') -OutputPath $candidatePath
    if ($LASTEXITCODE -ne 0) { throw 'Third-party inventory regeneration failed' }
    # PowerShell 5.1 and 7 differ in JSON formatting and culture-aware sort
    # order. Compare the same package records independent of those details.
    & node.exe (Join-Path $PSScriptRoot 'Compare-DependencyInventory.js') $inventoryPath $candidatePath
    Assert-True ($LASTEXITCODE -eq 0) 'Dependency inventory is stale. Run scripts/Update-ThirdPartyInventory.ps1 and review the result.'
}
finally {
    $resolvedParent = [System.IO.Path]::GetFullPath($testRoot).TrimEnd('\') + '\'
    $resolvedCandidate = [System.IO.Path]::GetFullPath($candidatePath)
    if ($resolvedCandidate.StartsWith($resolvedParent, [System.StringComparison]::OrdinalIgnoreCase) -and (Test-Path -LiteralPath $resolvedCandidate)) {
        Remove-Item -LiteralPath $resolvedCandidate -Force
    }
}

$inventory = Get-Content -LiteralPath $inventoryPath -Raw | ConvertFrom-Json
Assert-True ($inventory.rust.Count -gt 0) 'Rust dependency inventory is empty'
Assert-True ($inventory.npm.Count -gt 0) 'npm dependency inventory is empty'
Assert-True ($inventory.python.Count -eq $mapperLock.dependencies.packageCount) 'Python dependency inventory count differs from runtime lock'

$unknown = @(
    @($inventory.rust + $inventory.npm + $inventory.python) | Where-Object {
        [string]::IsNullOrWhiteSpace($_.license) -or $_.license -match '(?i)unknown|unlicensed|see license'
    }
)
$unknownNames = @($unknown | ForEach-Object { $_.name }) -join ', '
Assert-True ($unknown.Count -eq 0) "Unknown dependency licenses remain: $unknownNames"

$notices = Get-Content -LiteralPath (Join-Path $repoRoot 'THIRD_PARTY_NOTICES.md') -Raw
foreach ($requiredNotice in @('Mapperatorinator', 'slider', 'FFmpeg', 'Work in Progress', 'affiliated with')) {
    Assert-True ($notices.Contains($requiredNotice)) "Third-party notices omit required disclosure: $requiredNotice"
}

Write-Host "Third-party license gate passed: $($inventory.rust.Count) Rust, $($inventory.npm.Count) npm, $($inventory.python.Count) Python packages." -ForegroundColor Green
