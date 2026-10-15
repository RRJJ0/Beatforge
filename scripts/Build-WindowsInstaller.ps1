[CmdletBinding()]
param(
    [switch]$AllowUnsigned,
    [switch]$AllowDirty,
    [switch]$SkipTests,
    [switch]$TimestampUsesRfc3161,
    [string]$OutputDirectory,
    [string]$CertificateThumbprint = $env:BEATFORGE_WINDOWS_CERTIFICATE_THUMBPRINT,
    [string]$TimestampUrl = $env:BEATFORGE_WINDOWS_TIMESTAMP_URL
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$appRoot = Join-Path $repoRoot 'app'
$rustRoot = Join-Path $appRoot 'src-tauri'
$baseConfigPath = Join-Path $rustRoot 'tauri.conf.json'
$windowsConfigPath = Join-Path $rustRoot 'tauri.windows.conf.json'
$transientRoot = Join-Path $rustRoot '.release'
$signingConfigPath = Join-Path $transientRoot "signing-$PID.json"
$stagingDirectory = $null

function Assert-Tool([string]$Name) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Required release tool is unavailable: $Name"
    }
}

function Invoke-Checked([string]$Label, [scriptblock]$Action) {
    Write-Host "`n==> $Label" -ForegroundColor Cyan
    $global:LASTEXITCODE = 0
    & $Action
    if ($global:LASTEXITCODE -ne 0) { throw "$Label failed with exit code $global:LASTEXITCODE" }
}

function Get-CommandText([scriptblock]$Action) {
    $value = & $Action
    if ($LASTEXITCODE -ne 0) { throw 'Unable to read build tool version' }
    ($value | Out-String).Trim()
}

if ($env:OS -ne 'Windows_NT') { throw 'Windows installers must be built on Windows' }
foreach ($tool in @('git.exe', 'node.exe', 'pnpm.cmd', 'cargo.exe', 'rustc.exe')) { Assert-Tool $tool }

$baseConfig = Get-Content -LiteralPath $baseConfigPath -Raw | ConvertFrom-Json
$cargoManifest = Get-Content -LiteralPath (Join-Path $rustRoot 'Cargo.toml') -Raw
if ($cargoManifest -notmatch '(?m)^version = "([^"]+)"$') { throw 'Unable to read Cargo package version' }
$cargoVersion = $Matches[1]
if ($baseConfig.version -ne $cargoVersion) { throw "Version mismatch: Tauri $($baseConfig.version), Cargo $cargoVersion" }

$gitCommit = (& git.exe -C $repoRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $gitCommit -notmatch '^[0-9a-f]{40}$') { throw 'Unable to resolve the source commit' }
$gitStatus = @(& git.exe -C $repoRoot status --porcelain)
$sourceDirty = $gitStatus.Count -gt 0
if ($sourceDirty -and -not $AllowDirty) { throw 'Refusing a release build from a dirty worktree. Commit changes or pass -AllowDirty for a local smoke build.' }

$signedBuild = -not $AllowUnsigned
$normalizedThumbprint = ($CertificateThumbprint -replace '\s', '').ToUpperInvariant()
if ($signedBuild) {
    if ($normalizedThumbprint -notmatch '^[0-9A-F]{40}$') { throw 'Set BEATFORGE_WINDOWS_CERTIFICATE_THUMBPRINT to a 40-character SHA-1 certificate thumbprint.' }
    if ([string]::IsNullOrWhiteSpace($TimestampUrl) -or $TimestampUrl -notmatch '^https?://') { throw 'Set BEATFORGE_WINDOWS_TIMESTAMP_URL to the certificate provider timestamp URL.' }
    $certificate = Get-ChildItem Cert:\CurrentUser\My, Cert:\LocalMachine\My | Where-Object { $_.Thumbprint -eq $normalizedThumbprint } | Select-Object -First 1
    if ($null -eq $certificate) { throw "Code-signing certificate not found: $normalizedThumbprint" }
    if (-not $certificate.HasPrivateKey) { throw 'The selected code-signing certificate has no accessible private key' }
    if ($certificate.NotAfter -le (Get-Date)) { throw 'The selected code-signing certificate has expired' }
    $codeSigningEku = @($certificate.EnhancedKeyUsageList | Where-Object { $_.ObjectId.Value -eq '1.3.6.1.5.5.7.3.3' })
    if ($codeSigningEku.Count -eq 0) { throw 'The selected certificate is not valid for code signing' }
}

if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $shortCommit = $gitCommit.Substring(0, 12)
    $OutputDirectory = Join-Path $repoRoot "artifacts\windows\Beatforge-$($baseConfig.version)-$shortCommit"
}
$finalDirectory = [System.IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $finalDirectory) { throw "Artifact directory already exists and will not be overwritten: $finalDirectory" }
$stagingDirectory = $finalDirectory + ".staging-$PID"
if (Test-Path -LiteralPath $stagingDirectory) { throw "Staging directory already exists: $stagingDirectory" }

try {
    New-Item -ItemType Directory -Path $stagingDirectory -Force | Out-Null
    if ($signedBuild) {
        New-Item -ItemType Directory -Path $transientRoot -Force | Out-Null
        $signingConfig = [ordered]@{
            bundle = [ordered]@{
                windows = [ordered]@{
                    certificateThumbprint = $normalizedThumbprint
                    digestAlgorithm = 'sha256'
                    timestampUrl = $TimestampUrl
                    tsp = [bool]$TimestampUsesRfc3161
                }
            }
        }
        [System.IO.File]::WriteAllText($signingConfigPath, ($signingConfig | ConvertTo-Json -Depth 5), [System.Text.UTF8Encoding]::new($false))
    }

    if (-not $SkipTests) {
        Invoke-Checked 'GPU-free release gate' { & (Join-Path $PSScriptRoot 'Test-Release.ps1') -SkipBuild }
    }
    Invoke-Checked 'Windows installer policy' { & (Join-Path $PSScriptRoot 'Test-WindowsInstaller.ps1') }

    Push-Location $appRoot
    try {
        $arguments = @('tauri', 'build', '--bundles', 'nsis', '--ci')
        if ($signedBuild) { $arguments += @('--config', $signingConfigPath) } else { $arguments += '--no-sign' }
        Invoke-Checked 'Tauri NSIS build' { & pnpm.cmd @arguments }
    }
    finally {
        Pop-Location
    }

    $installerDirectory = Join-Path $rustRoot 'target\release\bundle\nsis'
    $installer = Get-ChildItem -LiteralPath $installerDirectory -File -Filter '*-setup.exe' | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1
    $application = Join-Path $rustRoot 'target\release\Beatforge.exe'
    if ($null -eq $installer) { throw 'Tauri did not produce an NSIS setup executable' }
    if (-not (Test-Path -LiteralPath $application -PathType Leaf)) { throw 'Tauri did not produce Beatforge.exe' }

    Copy-Item -LiteralPath $installer.FullName -Destination (Join-Path $stagingDirectory $installer.Name)
    Copy-Item -LiteralPath $application -Destination (Join-Path $stagingDirectory 'Beatforge.exe')
    Copy-Item -LiteralPath (Join-Path $repoRoot 'LICENSE') -Destination (Join-Path $stagingDirectory 'LICENSE')
    Copy-Item -LiteralPath (Join-Path $repoRoot 'THIRD_PARTY_NOTICES.md') -Destination (Join-Path $stagingDirectory 'THIRD_PARTY_NOTICES.md')
    Copy-Item -LiteralPath (Join-Path $repoRoot 'third_party\dependency-inventory.json') -Destination (Join-Path $stagingDirectory 'dependency-inventory.json')

    $executableFiles = @(Get-ChildItem -LiteralPath $stagingDirectory -File -Filter '*.exe')
    $signatureRecords = @(
        foreach ($file in $executableFiles | Sort-Object Name) {
            $signature = Get-AuthenticodeSignature -LiteralPath $file.FullName
            if ($signedBuild -and $signature.Status -ne 'Valid') { throw "Invalid Authenticode signature: $($file.Name) ($($signature.Status))" }
            [ordered]@{
                file = $file.Name
                status = [string]$signature.Status
                signerSubject = if ($null -ne $signature.SignerCertificate) { $signature.SignerCertificate.Subject } else { $null }
                signerThumbprint = if ($null -ne $signature.SignerCertificate) { $signature.SignerCertificate.Thumbprint } else { $null }
            }
        }
    )

    $hashFiles = @(Get-ChildItem -LiteralPath $stagingDirectory -File | Sort-Object Name)
    $checksumLines = @(
        foreach ($file in $hashFiles) {
            $hash = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            "$hash  $($file.Name)"
        }
    )
    [System.IO.File]::WriteAllLines((Join-Path $stagingDirectory 'SHA256SUMS.txt'), $checksumLines, [System.Text.UTF8Encoding]::new($false))

    $provenance = [ordered]@{
        schemaVersion = 1
        product = 'Beatforge'
        version = [string]$baseConfig.version
        target = 'x86_64-pc-windows-msvc'
        installerType = 'nsis-current-user'
        gitCommit = $gitCommit
        sourceDirty = $sourceDirty
        builtAtUtc = [DateTimeOffset]::UtcNow.ToString('O')
        signed = $signedBuild
        tools = [ordered]@{
            node = Get-CommandText { & node.exe --version }
            pnpm = Get-CommandText { & pnpm.cmd --version }
            rustc = Get-CommandText { & rustc.exe --version }
            cargo = Get-CommandText { & cargo.exe --version }
        }
        inputs = [ordered]@{
            tauriConfigSha256 = (Get-FileHash -LiteralPath $baseConfigPath -Algorithm SHA256).Hash.ToLowerInvariant()
            windowsConfigSha256 = (Get-FileHash -LiteralPath $windowsConfigPath -Algorithm SHA256).Hash.ToLowerInvariant()
            cargoLockSha256 = (Get-FileHash -LiteralPath (Join-Path $rustRoot 'Cargo.lock') -Algorithm SHA256).Hash.ToLowerInvariant()
            pnpmLockSha256 = (Get-FileHash -LiteralPath (Join-Path $appRoot 'pnpm-lock.yaml') -Algorithm SHA256).Hash.ToLowerInvariant()
            dependencyInventorySha256 = (Get-FileHash -LiteralPath (Join-Path $repoRoot 'third_party\dependency-inventory.json') -Algorithm SHA256).Hash.ToLowerInvariant()
        }
        signatures = $signatureRecords
        artifacts = @(
            foreach ($file in $hashFiles) {
                [ordered]@{
                    file = $file.Name
                    sizeBytes = $file.Length
                    sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
                }
            }
        )
    }
    [System.IO.File]::WriteAllText((Join-Path $stagingDirectory 'build-provenance.json'), ($provenance | ConvertTo-Json -Depth 8) + "`n", [System.Text.UTF8Encoding]::new($false))

    & (Join-Path $PSScriptRoot 'Test-WindowsInstaller.ps1') -ArtifactDirectory $stagingDirectory -RequireSignature:$signedBuild
    if ($LASTEXITCODE -ne 0) { throw 'Generated artifact verification failed' }

    $finalParent = Split-Path -Parent $finalDirectory
    if (-not (Test-Path -LiteralPath $finalParent)) { New-Item -ItemType Directory -Path $finalParent -Force | Out-Null }
    Move-Item -LiteralPath $stagingDirectory -Destination $finalDirectory
    $stagingDirectory = $null
    Write-Host "`nWindows release artifact ready: $finalDirectory" -ForegroundColor Green
}
finally {
    if (Test-Path -LiteralPath $signingConfigPath) { Remove-Item -LiteralPath $signingConfigPath -Force }
    if ((Test-Path -LiteralPath $transientRoot) -and -not (Get-ChildItem -LiteralPath $transientRoot -Force)) { Remove-Item -LiteralPath $transientRoot -Force }
    if ($null -ne $stagingDirectory -and (Test-Path -LiteralPath $stagingDirectory)) { Remove-Item -LiteralPath $stagingDirectory -Recurse -Force }
}
