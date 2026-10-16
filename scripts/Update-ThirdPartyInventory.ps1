[CmdletBinding()]
param(
    [string]$OutputPath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$appRoot = Join-Path $repoRoot 'app'
$rustManifest = Join-Path $appRoot 'src-tauri\Cargo.toml'
$requirementsPath = Join-Path $appRoot 'src-tauri\resources\python\mapperatorinator-windows-py310.requirements.txt'
$pythonLicensePath = Join-Path $repoRoot 'third_party\python-licenses.json'
$componentPolicyPath = Join-Path $repoRoot 'third_party\component-policy.json'

if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $repoRoot 'third_party\dependency-inventory.json'
}
$OutputPath = [System.IO.Path]::GetFullPath($OutputPath)

foreach ($tool in @('cargo.exe', 'node.exe', 'pnpm.cmd')) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        throw "Required inventory tool is unavailable: $tool"
    }
}

function Get-FileSha256([string]$Path) {
    # Git may check text files out with CRLF on Windows. Inventory hashes are
    # based on the repository's LF-normalized content, not checkout settings.
    $content = [System.IO.File]::ReadAllText($Path, [System.Text.Encoding]::UTF8).Replace("`r`n", "`n")
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($content)
    $hasher = [System.Security.Cryptography.SHA256]::Create()
    try {
        ([System.BitConverter]::ToString($hasher.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
    }
    finally {
        $hasher.Dispose()
    }
}

function Assert-License([string]$Name, [string]$License) {
    if ([string]::IsNullOrWhiteSpace($License) -or $License -match '(?i)unknown|unlicensed|see license') {
        throw "Dependency has no reviewable declared license: $Name ($License)"
    }
}

# Windows PowerShell 5.1 cannot parse Cargo's full JSON when nested metadata
# contains keys that differ only by case. Its native stdout decoding can also
# corrupt Unicode author names. Let Node invoke Cargo and project its fields.
$cargoPackageJson = & node.exe (Join-Path $PSScriptRoot 'Project-CargoPackages.js') $rustManifest
if ($LASTEXITCODE -ne 0) { throw 'cargo package metadata projection failed' }
$cargoMetadata = $cargoPackageJson | ConvertFrom-Json
$rustPackages = @(
    foreach ($package in $cargoMetadata.packages) {
        $license = [string]$package.license
        Assert-License "$($package.name)@$($package.version)" $license
        [ordered]@{
            name = [string]$package.name
            version = [string]$package.version
            license = $license
            authors = @($package.authors)
            repository = [string]$package.repository
            source = [string]$package.source
        }
    }
) | Sort-Object { $_.name }, { $_.version }, { $_.source }

Push-Location $appRoot
try {
    $npmJson = & pnpm.cmd licenses list --prod --json
    if ($LASTEXITCODE -ne 0) { throw 'pnpm license inventory failed' }
}
finally {
    Pop-Location
}
$npmReport = $npmJson | ConvertFrom-Json
$npmPackages = @(
    foreach ($licenseGroup in $npmReport.PSObject.Properties) {
        foreach ($package in $licenseGroup.Value) {
            $license = [string]$package.license
            foreach ($version in $package.versions) {
                Assert-License "$($package.name)@$version" $license
                $authorProperty = $package.PSObject.Properties['author']
                $authorValue = if ($null -ne $authorProperty) { $authorProperty.Value } else { $null }
                $author = if ($authorValue -is [string]) { $authorValue } elseif ($null -ne $authorValue) { $authorValue.name } else { '' }
                $homepageProperty = $package.PSObject.Properties['homepage']
                $homepage = if ($null -ne $homepageProperty) { [string]$homepageProperty.Value } else { '' }
                [ordered]@{
                    name = [string]$package.name
                    version = [string]$version
                    license = $license
                    author = [string]$author
                    homepage = $homepage
                }
            }
        }
    }
) | Sort-Object { $_.name }, { $_.version }

$pythonCatalog = (Get-Content -LiteralPath $pythonLicensePath -Raw | ConvertFrom-Json).packages
$pythonByName = @{}
foreach ($package in $pythonCatalog) {
    $pythonByName[$package.name.ToLowerInvariant()] = $package
}
$lockedPython = [System.Collections.Generic.List[object]]::new()
foreach ($line in Get-Content -LiteralPath $requirementsPath) {
    if ($line -match '^([A-Za-z0-9_.-]+)==([^\s\\]+)') {
        $name = $Matches[1].ToLowerInvariant()
        $version = $Matches[2]
    }
    elseif ($line -match '^([A-Za-z0-9_.-]+)\s*@') {
        $name = $Matches[1].ToLowerInvariant()
        $version = '0.8.2'
    }
    else {
        continue
    }
    if (-not $pythonByName.ContainsKey($name)) { throw "Missing reviewed Python license: $name@$version" }
    $reviewed = $pythonByName[$name]
    if ($reviewed.version -ne $version) { throw "Python license mapping is stale: $name is $version, mapping is $($reviewed.version)" }
    Assert-License "$name@$version" $reviewed.license
    $lockedPython.Add([ordered]@{
        name = $name
        version = $version
        license = [string]$reviewed.license
        source = "https://pypi.org/project/$name/$version/"
    })
}
if ($lockedPython.Count -ne $pythonCatalog.Count) {
    throw "Python inventory count mismatch: lock has $($lockedPython.Count), review has $($pythonCatalog.Count)"
}

$components = (Get-Content -LiteralPath $componentPolicyPath -Raw | ConvertFrom-Json).components
foreach ($component in $components) {
    Assert-License $component.name $component.license
    if ($component.distribution -notmatch '^download_only') {
        throw "Unreviewed distribution boundary for $($component.name): $($component.distribution)"
    }
}

$inventory = [ordered]@{
    schemaVersion = 1
    generatedFrom = [ordered]@{
        cargoLockSha256 = Get-FileSha256 (Join-Path $appRoot 'src-tauri\Cargo.lock')
        pnpmLockSha256 = Get-FileSha256 (Join-Path $appRoot 'pnpm-lock.yaml')
        pythonRequirementsSha256 = Get-FileSha256 $requirementsPath
        mapperatorinatorLockSha256 = Get-FileSha256 (Join-Path $appRoot 'src-tauri\resources\manifests\mapperatorinator-v32.lock.json')
        pythonLicenseReviewSha256 = Get-FileSha256 $pythonLicensePath
        componentPolicySha256 = Get-FileSha256 $componentPolicyPath
    }
    distributionBoundary = [ordered]@{
        installer = @('Beatforge desktop binary', 'compiled frontend assets', 'application icons', 'LICENSE', 'THIRD_PARTY_NOTICES.md', 'dependency-inventory.json')
        managedRuntime = 'All Python, AI, model and FFmpeg components are downloaded after installation and are forbidden from installer artifacts.'
    }
    rust = @($rustPackages)
    npm = @($npmPackages)
    python = @($lockedPython | Sort-Object { $_.name }, { $_.version })
    managedComponents = @($components | Sort-Object name)
}

$parent = Split-Path -Parent $OutputPath
if (-not (Test-Path -LiteralPath $parent)) { New-Item -ItemType Directory -Path $parent | Out-Null }
$json = $inventory | ConvertTo-Json -Depth 8
[System.IO.File]::WriteAllText($OutputPath, $json + "`n", [System.Text.UTF8Encoding]::new($false))
Write-Host "Wrote third-party inventory: $OutputPath" -ForegroundColor Green
