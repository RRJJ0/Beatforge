[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$NewArtifactDirectory,
    [Parameter(Mandatory)][string]$ManifestUrl,
    [Parameter(Mandatory)][string]$InstallerUrl,
    [string]$OldVersion = '0.0.9',
    [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-HttpsUrl([string]$Value, [string]$Label) {
    $parsed = $null
    if (-not [Uri]::TryCreate($Value, [UriKind]::Absolute, [ref]$parsed) -or $parsed.Scheme -ne 'https') {
        throw "$Label must be an absolute HTTPS URL"
    }
    return $parsed
}

function Assert-PlainVersion([string]$Value, [string]$Label) {
    if ($Value -notmatch '^\d+\.\d+\.\d+$') { throw "$Label must be a three-part SemVer version" }
    return [version]$Value
}

if ($env:OS -ne 'Windows_NT') { throw 'The update acceptance installer must be built on Windows' }
foreach ($tool in @('pnpm.cmd', 'cargo.exe', 'git.exe')) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) { throw "Required build tool is unavailable: $tool" }
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$appRoot = Join-Path $repoRoot 'app'
$baseConfigPath = Join-Path $appRoot 'src-tauri\tauri.conf.json'
$baseConfig = Get-Content -LiteralPath $baseConfigPath -Raw | ConvertFrom-Json
$productionEndpoint = [string]$baseConfig.plugins.updater.endpoints[0]
$manifestUri = Assert-HttpsUrl $ManifestUrl 'ManifestUrl'
$installerUri = Assert-HttpsUrl $InstallerUrl 'InstallerUrl'
if ($ManifestUrl -eq $productionEndpoint) { throw 'The acceptance build must not use the production updater endpoint' }
if ($manifestUri.AbsolutePath -notmatch '/latest\.json$') { throw 'ManifestUrl must end in /latest.json' }
if ($manifestUri.Host -eq 'github.com' -and $manifestUri.AbsolutePath -match '^/RRJJ0/Beatforge/releases/') {
    throw 'The acceptance manifest must not be hosted in the production Beatforge release channel'
}
if ($installerUri.Host -eq 'github.com' -and $installerUri.AbsolutePath -match '^/RRJJ0/Beatforge/releases/') {
    throw 'The acceptance installer must not be hosted in the production Beatforge release channel'
}

$artifactRoot = [System.IO.Path]::GetFullPath($NewArtifactDirectory)
$provenancePath = Join-Path $artifactRoot 'build-provenance.json'
if (-not (Test-Path -LiteralPath $provenancePath -PathType Leaf)) { throw 'NewArtifactDirectory has no build provenance' }
$provenance = Get-Content -LiteralPath $provenancePath -Raw | ConvertFrom-Json
$newVersion = [string]$provenance.version
if ((Assert-PlainVersion $OldVersion 'OldVersion') -ge (Assert-PlainVersion $newVersion 'New artifact version')) {
    throw 'OldVersion must be lower than the signed new artifact version'
}
if ($newVersion -ne [string]$baseConfig.version) {
    throw 'The new artifact version must match this checkout for signature verification'
}
if ($provenance.releaseMode -notin @('signed', 'unsigned-public')) {
    throw 'The target must be a publishable, updater-signed artifact, not a local smoke build'
}

& (Join-Path $PSScriptRoot 'Test-WindowsInstaller.ps1') -ArtifactDirectory $artifactRoot -RequireSignature:([bool]$provenance.signed) -RequirePublicRelease
if ($LASTEXITCODE -ne 0) { throw 'New artifact verification failed' }

$newInstaller = @(Get-ChildItem -LiteralPath $artifactRoot -File -Filter '*-setup.exe')
if ($newInstaller.Count -ne 1) { throw 'Expected exactly one new installer' }
$newSignature = Join-Path $artifactRoot ($newInstaller[0].Name + '.sig')
if (-not (Test-Path -LiteralPath $newSignature -PathType Leaf)) { throw 'New installer updater signature is missing' }
if ($installerUri.Segments[-1] -ne $newInstaller[0].Name) {
    throw 'InstallerUrl must end with the exact new installer filename'
}

if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $repoRoot ('artifacts\update-acceptance\' + [DateTimeOffset]::UtcNow.ToString('yyyyMMdd-HHmmss'))
}
$outputRoot = [System.IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $outputRoot) { throw "OutputDirectory already exists: $outputRoot" }
$hostingRoot = Join-Path $outputRoot 'hosting'
$oldRoot = Join-Path $outputRoot 'old'
New-Item -ItemType Directory -Path $hostingRoot, $oldRoot -Force | Out-Null

# This overlay is ignored with artifacts/ and never changes the checked-in release endpoint or key.
$overlayPath = Join-Path $outputRoot 'old-version-overlay.json'
$overlay = [ordered]@{
    version = $OldVersion
    bundle = [ordered]@{ createUpdaterArtifacts = $false }
    plugins = [ordered]@{
        updater = [ordered]@{ endpoints = @($ManifestUrl) }
    }
}
[System.IO.File]::WriteAllText($overlayPath, ($overlay | ConvertTo-Json -Depth 8) + "`n", [System.Text.UTF8Encoding]::new($false))

$buildStarted = [DateTime]::UtcNow
Push-Location $appRoot
try {
    & pnpm.cmd tauri build --bundles nsis --ci --no-sign --config $overlayPath
    if ($LASTEXITCODE -ne 0) { throw "Old-version acceptance build failed with exit code $LASTEXITCODE" }
}
finally {
    Pop-Location
}

$bundleRoot = Join-Path $appRoot 'src-tauri\target\release\bundle\nsis'
$oldInstallers = @(Get-ChildItem -LiteralPath $bundleRoot -File -Filter '*-setup.exe' |
    Where-Object { $_.Name -match ('^Beatforge_' + [regex]::Escape($OldVersion) + '_') -and $_.LastWriteTimeUtc -ge $buildStarted.AddSeconds(-5) })
if ($oldInstallers.Count -ne 1) { throw 'Could not identify exactly one fresh old-version installer' }
$builtExecutable = Join-Path $appRoot 'src-tauri\target\release\Beatforge.exe'
$builtVersion = (Get-Item -LiteralPath $builtExecutable).VersionInfo.ProductVersion
if ($builtVersion -ne $OldVersion) { throw "Built executable version is $builtVersion, expected $OldVersion" }
if (-not [System.Text.Encoding]::UTF8.GetString([System.IO.File]::ReadAllBytes($builtExecutable)).Contains($ManifestUrl)) {
    throw 'Built executable does not contain the isolated updater endpoint'
}
Copy-Item -LiteralPath $oldInstallers[0].FullName -Destination (Join-Path $oldRoot $oldInstallers[0].Name)
Copy-Item -LiteralPath $newInstaller[0].FullName -Destination (Join-Path $hostingRoot $newInstaller[0].Name)
Copy-Item -LiteralPath $newSignature -Destination (Join-Path $hostingRoot ($newInstaller[0].Name + '.sig'))
Copy-Item -LiteralPath (Join-Path $artifactRoot 'latest.json') -Destination (Join-Path $outputRoot 'original-candidate-latest.json')

$signature = (Get-Content -LiteralPath $newSignature -Raw).Trim()
$manifest = [ordered]@{
    version = $newVersion
    notes = "Beatforge isolated update acceptance $OldVersion -> $newVersion"
    pub_date = [DateTimeOffset]::UtcNow.ToString('O')
    platforms = [ordered]@{
        'windows-x86_64' = [ordered]@{
            signature = $signature
            url = $InstallerUrl
        }
    }
}
[System.IO.File]::WriteAllText((Join-Path $hostingRoot 'latest.json'), ($manifest | ConvertTo-Json -Depth 6) + "`n", [System.Text.UTF8Encoding]::new($false))

$record = [ordered]@{
    schemaVersion = 1
    createdAtUtc = [DateTimeOffset]::UtcNow.ToString('O')
    oldVersion = $OldVersion
    newVersion = $newVersion
    manifestUrl = $ManifestUrl
    installerUrl = $InstallerUrl
    oldInstaller = "old/$($oldInstallers[0].Name)"
    oldInstallerSha256 = (Get-FileHash -LiteralPath $oldInstallers[0].FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    newInstallerSha256 = (Get-FileHash -LiteralPath $newInstaller[0].FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    originalCandidateManifest = 'original-candidate-latest.json'
    newArtifactCommit = [string]$provenance.gitCommit
    sourceCommit = (& git.exe -C $repoRoot rev-parse HEAD).Trim()
    sourceDirty = @(& git.exe -C $repoRoot status --porcelain).Count -gt 0
    result = 'pending-manual-update'
}
[System.IO.File]::WriteAllText((Join-Path $outputRoot 'acceptance.json'), ($record | ConvertTo-Json -Depth 5) + "`n", [System.Text.UTF8Encoding]::new($false))
foreach ($scriptName in @('Start-UpdateAcceptanceServer.ps1', 'Serve-UpdateAcceptanceHttps.mjs', 'Test-UpdateAcceptanceEndpoint.ps1')) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $scriptName) -Destination (Join-Path $outputRoot $scriptName)
}

Write-Host "Isolated acceptance bundle ready: $outputRoot" -ForegroundColor Green
Write-Host 'Copy the whole bundle to a disposable Windows account or VM, then start its localhost HTTPS server.'
Write-Host 'Install old/ only in that test environment; do not publish these files in the production release.'
