[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$BundleDirectory,
    [switch]$DisposableEnvironment,
    [int]$Port = 48765
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if (-not $DisposableEnvironment) {
    throw 'This script temporarily trusts a local HTTPS certificate. Run only in a disposable Windows account or VM and pass -DisposableEnvironment.'
}
if ($env:OS -ne 'Windows_NT') { throw 'This test server requires Windows' }
if ($Port -lt 1024 -or $Port -gt 65535) { throw 'Port must be between 1024 and 65535' }
if (-not (Get-Command node.exe -ErrorAction SilentlyContinue)) { throw 'Node.js is required in the disposable test account or VM' }

$bundleRoot = [System.IO.Path]::GetFullPath($BundleDirectory)
$recordPath = Join-Path $bundleRoot 'acceptance.json'
$serverPath = Join-Path $bundleRoot 'Serve-UpdateAcceptanceHttps.mjs'
if (-not (Test-Path -LiteralPath $recordPath -PathType Leaf)) { throw 'Acceptance record is missing' }
if (-not (Test-Path -LiteralPath $serverPath -PathType Leaf)) { throw 'HTTPS server script is missing' }
$record = Get-Content -LiteralPath $recordPath -Raw | ConvertFrom-Json
$expectedBase = "https://localhost:$Port/"
if ($record.manifestUrl -ne ($expectedBase + 'latest.json')) { throw "ManifestUrl must be $($expectedBase)latest.json" }
$installerName = [System.IO.Path]::GetFileName(([Uri]$record.installerUrl).AbsolutePath)
if ($record.installerUrl -ne ($expectedBase + $installerName)) { throw 'InstallerUrl must use the same local HTTPS port' }
foreach ($file in @('latest.json', $installerName, ($installerName + '.sig'))) {
    if (-not (Test-Path -LiteralPath (Join-Path $bundleRoot ('hosting\' + $file)) -PathType Leaf)) {
        throw "Missing local HTTPS payload: $file"
    }
}

$installRoot = Join-Path $env:LOCALAPPDATA 'Beatforge'
$dataRoot = Join-Path $env:APPDATA 'com.beatforge.desktop'
if (Test-Path -LiteralPath $installRoot) { throw "Existing Beatforge installation detected: $installRoot" }
if (Test-Path -LiteralPath $dataRoot) { throw "Existing Beatforge user data detected: $dataRoot" }

$certificate = $null
$publicCertificate = $null
$rootTrusted = $false
$pfxPath = Join-Path $env:TEMP ('beatforge-update-acceptance-' + [guid]::NewGuid().ToString('N') + '.pfx')
$previousPassword = $env:BEATFORGE_UPDATE_ACCEPTANCE_PFX_PASSWORD
$previousPath = $env:BEATFORGE_UPDATE_ACCEPTANCE_PFX_PATH
try {
    $certificate = New-SelfSignedCertificate -DnsName 'localhost' -Type SSLServerAuthentication `
        -CertStoreLocation 'Cert:\CurrentUser\My' -KeyExportPolicy Exportable -NotAfter (Get-Date).AddDays(2)
    $publicCertificate = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new(
        $certificate.Export([System.Security.Cryptography.X509Certificates.X509ContentType]::Cert))
    $rootStore = [System.Security.Cryptography.X509Certificates.X509Store]::new('Root', 'CurrentUser')
    try {
        $rootStore.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
        $rootStore.Add($publicCertificate)
        $rootTrusted = $true
    }
    finally { $rootStore.Close() }

    $randomBytes = New-Object byte[] 32
    $generator = [System.Security.Cryptography.RandomNumberGenerator]::Create()
    try { $generator.GetBytes($randomBytes) } finally { $generator.Dispose() }
    $password = [Convert]::ToBase64String($randomBytes)
    $securePassword = ConvertTo-SecureString -String $password -AsPlainText -Force
    Export-PfxCertificate -Cert $certificate -FilePath $pfxPath -Password $securePassword | Out-Null
    $env:BEATFORGE_UPDATE_ACCEPTANCE_PFX_PASSWORD = $password
    $env:BEATFORGE_UPDATE_ACCEPTANCE_PFX_PATH = $pfxPath

    Write-Host "Temporary localhost certificate: $($certificate.Thumbprint)" -ForegroundColor Yellow
    Write-Host 'It is trusted only in this Windows account while this server runs. Press Ctrl+C after testing.' -ForegroundColor Yellow
    & node.exe $serverPath $bundleRoot $Port
    if ($LASTEXITCODE -ne 0) { throw "HTTPS server exited with code $LASTEXITCODE" }
}
finally {
    $env:BEATFORGE_UPDATE_ACCEPTANCE_PFX_PASSWORD = $previousPassword
    $env:BEATFORGE_UPDATE_ACCEPTANCE_PFX_PATH = $previousPath
    if ($rootTrusted) {
        $rootStore = [System.Security.Cryptography.X509Certificates.X509Store]::new('Root', 'CurrentUser')
        try {
            $rootStore.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
            $rootStore.Remove($publicCertificate)
        }
        catch { Write-Warning "Could not remove temporary trusted certificate $($certificate.Thumbprint): $_" }
        finally { $rootStore.Close() }
    }
    if ($null -ne $certificate) {
        $myStore = [System.Security.Cryptography.X509Certificates.X509Store]::new('My', 'CurrentUser')
        try {
            $myStore.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
            $myStore.Remove($certificate)
        }
        catch { Write-Warning "Could not remove temporary private certificate $($certificate.Thumbprint): $_" }
        finally { $myStore.Close() }
    }
    if (Test-Path -LiteralPath $pfxPath) { Remove-Item -LiteralPath $pfxPath -Force }
}
