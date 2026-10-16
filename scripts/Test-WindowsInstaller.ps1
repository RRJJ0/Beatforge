[CmdletBinding()]
param(
    [string]$ArtifactDirectory,
    [switch]$RequireSignature,
    [switch]$RequirePublicRelease
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$baseConfigPath = Join-Path $repoRoot 'app\src-tauri\tauri.conf.json'
$windowsConfigPath = Join-Path $repoRoot 'app\src-tauri\tauri.windows.conf.json'

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

$base = Get-Content -LiteralPath $baseConfigPath -Raw | ConvertFrom-Json
$windows = Get-Content -LiteralPath $windowsConfigPath -Raw | ConvertFrom-Json
$releaseNotesPath = Join-Path $repoRoot 'docs\release\release-notes.md'
$releaseNotes = Get-Content -LiteralPath $releaseNotesPath -Raw
Assert-True ($releaseNotes -match ('(?m)^# Beatforge ' + [regex]::Escape([string]$base.version) + ' release candidate notes$')) 'Release notes version must match the application version'
foreach ($document in @('docs\release\release-notes.zh-TW.md', 'docs\release\known-issues.md', 'docs\release\known-issues.zh-TW.md', 'docs\release\release-candidate-runbook.md', 'docs\testing\stage-9-gpu-support-matrix.md', 'docs\testing\stage-9-gpu-support-matrix.zh-TW.md')) {
    Assert-True (Test-Path -LiteralPath (Join-Path $repoRoot $document) -PathType Leaf) "Release document is missing: $document"
}

Assert-True ($base.productName -eq 'Beatforge') 'Unexpected Windows product name'
Assert-True ($base.mainBinaryName -eq 'Beatforge') 'The installed executable must be named Beatforge.exe'
Assert-True ($base.identifier -eq 'com.beatforge.desktop') 'The stable application identifier changed'
Assert-True ($base.bundle.license -eq 'MIT') 'Bundle metadata must declare MIT'
Assert-True ($base.bundle.licenseFile -eq '../../LICENSE') 'Bundle metadata must include the project license'
Assert-True ($base.bundle.createUpdaterArtifacts -eq $true) 'Signed updater artifacts must be produced'
Assert-True (-not [string]::IsNullOrWhiteSpace($base.plugins.updater.pubkey)) 'Updater public key is missing'
Assert-True ($base.plugins.updater.requireSignedVersion -eq $true) 'Updater signatures must be bound to the announced version'
Assert-True ($base.plugins.updater.endpoints.Count -gt 0) 'Updater endpoint is missing'
Assert-True ($windows.bundle.targets.Count -eq 1 -and $windows.bundle.targets[0] -eq 'nsis') 'Windows release target must be NSIS only'
Assert-True ($windows.bundle.windows.nsis.installMode -eq 'currentUser') 'Installer must not require administrator access'
Assert-True ($windows.bundle.windows.allowDowngrades -eq $false) 'Older installers must not silently downgrade the app'
Assert-True ($windows.bundle.windows.bundleVCRuntime -eq $true) 'Installer must carry the required Visual C++ runtime DLLs'
Assert-True ($windows.bundle.windows.webviewInstallMode.type -eq 'downloadBootstrapper') 'WebView2 bootstrap policy changed without review'

$libSource = Get-Content -LiteralPath (Join-Path $repoRoot 'app\src-tauri\src\lib.rs') -Raw
Assert-True ($libSource.Contains('.app_data_dir()')) 'Managed user data is no longer rooted in the OS app-data directory'

if ([string]::IsNullOrWhiteSpace($ArtifactDirectory)) {
    Write-Host 'Windows installer configuration passed. Artifact verification was not requested.' -ForegroundColor Green
    exit 0
}

$artifactRoot = [System.IO.Path]::GetFullPath($ArtifactDirectory)
Assert-True (Test-Path -LiteralPath $artifactRoot -PathType Container) "Artifact directory does not exist: $artifactRoot"

$installers = @(Get-ChildItem -LiteralPath $artifactRoot -File -Filter '*-setup.exe')
$applications = @(Get-ChildItem -LiteralPath $artifactRoot -File -Filter 'Beatforge.exe')
$updaterSignatures = @(Get-ChildItem -LiteralPath $artifactRoot -File -Filter '*-setup.exe.sig')
Assert-True ($installers.Count -eq 1) 'Expected exactly one NSIS setup executable'
Assert-True ($applications.Count -eq 1) 'Expected exactly one standalone Beatforge executable'
Assert-True ($updaterSignatures.Count -eq 1) 'Expected exactly one signed updater artifact'

foreach ($required in @('LICENSE', 'THIRD_PARTY_NOTICES.md', 'dependency-inventory.json', 'RELEASE_NOTES.md', 'KNOWN_ISSUES.md', 'GPU_SUPPORT_MATRIX.md', 'RELEASE_CANDIDATE_RUNBOOK.md', 'latest.json', 'SHA256SUMS.txt', 'build-provenance.json')) {
    Assert-True (Test-Path -LiteralPath (Join-Path $artifactRoot $required) -PathType Leaf) "Release artifact is missing $required"
}
if ($RequirePublicRelease) {
    foreach ($required in @('RELEASE_NOTES.zh-TW.md', 'KNOWN_ISSUES.zh-TW.md', 'GPU_SUPPORT_MATRIX.zh-TW.md')) {
        Assert-True (Test-Path -LiteralPath (Join-Path $artifactRoot $required) -PathType Leaf) "Public release artifact is missing $required"
    }
}

$checksumLines = Get-Content -LiteralPath (Join-Path $artifactRoot 'SHA256SUMS.txt')
$checksumEntries = @{}
foreach ($line in $checksumLines) {
    if ($line -notmatch '^([0-9a-f]{64})  (.+)$') { throw "Malformed checksum line: $line" }
    Assert-True (-not $checksumEntries.ContainsKey($Matches[2])) "Duplicate checksum entry: $($Matches[2])"
    $checksumEntries[$Matches[2]] = $Matches[1]
}
$artifactFiles = @(Get-ChildItem -LiteralPath $artifactRoot -File)
$hashedFiles = @($artifactFiles | Where-Object { $_.Name -notin @('SHA256SUMS.txt', 'build-provenance.json') })
Assert-True ($checksumEntries.Count -eq $hashedFiles.Count) 'Checksum manifest has missing or extra entries'
foreach ($file in $hashedFiles) {
    Assert-True ($checksumEntries.ContainsKey($file.Name)) "No checksum recorded for $($file.Name)"
    $actual = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    Assert-True ($actual -eq $checksumEntries[$file.Name]) "Checksum mismatch for $($file.Name)"
}

$provenance = Get-Content -LiteralPath (Join-Path $artifactRoot 'build-provenance.json') -Raw | ConvertFrom-Json
Assert-True ($provenance.schemaVersion -eq 1) 'Unsupported build provenance schema'
Assert-True ($provenance.product -eq 'Beatforge') 'Unexpected product in build provenance'
Assert-True ($provenance.version -eq $base.version) 'Artifact version does not match Tauri configuration'
Assert-True ($provenance.gitCommit -match '^[0-9a-f]{40}$') 'Build provenance has no full Git commit'
Assert-True ($provenance.signed -eq $RequireSignature.IsPresent) 'Build provenance signing state differs from the required artifact state'
$releaseMode = if ($null -ne $provenance.PSObject.Properties['releaseMode']) {
    [string]$provenance.releaseMode
} elseif ($provenance.signed) {
    'signed'
} else {
    'local-smoke'
}
Assert-True ($releaseMode -in @('signed', 'unsigned-public', 'local-smoke')) 'Unknown release mode in build provenance'
Assert-True (($releaseMode -eq 'signed') -eq $RequireSignature.IsPresent) 'Release mode and Authenticode policy disagree'
if ($releaseMode -ne 'local-smoke') {
    Assert-True (-not $provenance.sourceDirty) 'A public release must come from a clean source revision'
}
if ($RequirePublicRelease) {
    Assert-True ($null -ne $provenance.PSObject.Properties['releaseMode']) 'Public releases require an explicit release mode'
    Assert-True ($releaseMode -ne 'local-smoke') 'Local smoke artifacts are not publishable'
}
if ($releaseMode -eq 'unsigned-public') {
    $notes = Get-Content -LiteralPath (Join-Path $artifactRoot 'RELEASE_NOTES.md') -Raw
    Assert-True ($notes.Contains('Windows Authenticode unsigned')) 'Unsigned public release notes must disclose the missing Windows Publisher signature'
}
Assert-True (@($provenance.artifacts).Count -eq $hashedFiles.Count) 'Build provenance artifact list is incomplete'
foreach ($file in $hashedFiles) {
    $record = @($provenance.artifacts | Where-Object { $_.file -eq $file.Name })
    Assert-True ($record.Count -eq 1) "Build provenance must contain exactly one entry for $($file.Name)"
    Assert-True ($record[0].sizeBytes -eq $file.Length) "Build provenance size mismatch for $($file.Name)"
    Assert-True ($record[0].sha256 -eq $checksumEntries[$file.Name]) "Build provenance hash mismatch for $($file.Name)"
}

$updateManifest = Get-Content -LiteralPath (Join-Path $artifactRoot 'latest.json') -Raw | ConvertFrom-Json
$windowsUpdate = $updateManifest.platforms.'windows-x86_64'
Assert-True ($updateManifest.version -eq $base.version) 'Update manifest version does not match the application'
Assert-True ($windowsUpdate.url -match '^https://') 'Update artifact URL must use HTTPS'
Assert-True ($windowsUpdate.url.EndsWith($installers[0].Name)) 'Update manifest URL does not name the generated installer'
Assert-True ($windowsUpdate.signature -eq (Get-Content -LiteralPath $updaterSignatures[0].FullName -Raw).Trim()) 'Update manifest signature differs from the generated signature'

Push-Location (Join-Path $repoRoot 'app\src-tauri')
try {
    & cargo.exe run --quiet --release --example verify_update_artifact -- $baseConfigPath $installers[0].FullName $updaterSignatures[0].FullName ([string]$base.version)
    if ($LASTEXITCODE -ne 0) { throw 'Updater artifact cryptographic signature verification failed' }
}
finally {
    Pop-Location
}

foreach ($executable in @($installers[0], $applications[0])) {
    $signature = Get-AuthenticodeSignature -LiteralPath $executable.FullName
    if ($RequireSignature) {
        Assert-True ($signature.Status -eq 'Valid') "Authenticode signature is not valid: $($executable.Name) ($($signature.Status))"
        $signatureRecord = @($provenance.signatures | Where-Object { $_.file -eq $executable.Name })
        Assert-True ($signatureRecord.Count -eq 1) "Missing signer provenance for $($executable.Name)"
        Assert-True ($signatureRecord[0].signerThumbprint -eq $signature.SignerCertificate.Thumbprint) "Signer changed for $($executable.Name)"
    }
    elseif ($signature.Status -notin @('Valid', 'NotSigned')) {
        throw "Unexpected Authenticode state: $($executable.Name) ($($signature.Status))"
    }
}

Assert-True ($installers[0].Length -lt 300MB) 'Installer is unexpectedly large; verify that runtime or model files were not bundled'
Write-Host "Windows installer artifact passed: $artifactRoot" -ForegroundColor Green
