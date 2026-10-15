[CmdletBinding()]
param(
    [string]$ArtifactDirectory,
    [switch]$RequireSignature
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

Assert-True ($base.productName -eq 'Beatforge') 'Unexpected Windows product name'
Assert-True ($base.mainBinaryName -eq 'Beatforge') 'The installed executable must be named Beatforge.exe'
Assert-True ($base.identifier -eq 'com.beatforge.desktop') 'The stable application identifier changed'
Assert-True ($base.bundle.license -eq 'MIT') 'Bundle metadata must declare MIT'
Assert-True ($base.bundle.licenseFile -eq '../../LICENSE') 'Bundle metadata must include the project license'
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
Assert-True ($installers.Count -eq 1) 'Expected exactly one NSIS setup executable'
Assert-True ($applications.Count -eq 1) 'Expected exactly one standalone Beatforge executable'

foreach ($required in @('LICENSE', 'THIRD_PARTY_NOTICES.md', 'dependency-inventory.json', 'SHA256SUMS.txt', 'build-provenance.json')) {
    Assert-True (Test-Path -LiteralPath (Join-Path $artifactRoot $required) -PathType Leaf) "Release artifact is missing $required"
}

$checksumLines = Get-Content -LiteralPath (Join-Path $artifactRoot 'SHA256SUMS.txt')
$checksumEntries = @{}
foreach ($line in $checksumLines) {
    if ($line -notmatch '^([0-9a-f]{64})  (.+)$') { throw "Malformed checksum line: $line" }
    $checksumEntries[$Matches[2]] = $Matches[1]
}
foreach ($file in Get-ChildItem -LiteralPath $artifactRoot -File | Where-Object { $_.Name -notin @('SHA256SUMS.txt', 'build-provenance.json') }) {
    Assert-True ($checksumEntries.ContainsKey($file.Name)) "No checksum recorded for $($file.Name)"
    $actual = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    Assert-True ($actual -eq $checksumEntries[$file.Name]) "Checksum mismatch for $($file.Name)"
}

$provenance = Get-Content -LiteralPath (Join-Path $artifactRoot 'build-provenance.json') -Raw | ConvertFrom-Json
Assert-True ($provenance.schemaVersion -eq 1) 'Unsupported build provenance schema'
Assert-True ($provenance.product -eq 'Beatforge') 'Unexpected product in build provenance'
Assert-True ($provenance.version -eq $base.version) 'Artifact version does not match Tauri configuration'
Assert-True ($provenance.gitCommit -match '^[0-9a-f]{40}$') 'Build provenance has no full Git commit'

foreach ($executable in @($installers[0], $applications[0])) {
    $signature = Get-AuthenticodeSignature -LiteralPath $executable.FullName
    if ($RequireSignature) {
        Assert-True ($signature.Status -eq 'Valid') "Authenticode signature is not valid: $($executable.Name) ($($signature.Status))"
    }
    elseif ($signature.Status -notin @('Valid', 'NotSigned')) {
        throw "Unexpected Authenticode state: $($executable.Name) ($($signature.Status))"
    }
}

Assert-True ($installers[0].Length -lt 300MB) 'Installer is unexpectedly large; verify that runtime or model files were not bundled'
Write-Host "Windows installer artifact passed: $artifactRoot" -ForegroundColor Green
