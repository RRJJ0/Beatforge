[CmdletBinding()]
param([Parameter(Mandatory)][string]$BundleDirectory)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-HttpsResponse($Response, [string]$Label) {
    $responseUri = if ($null -ne $Response.BaseResponse.PSObject.Properties['ResponseUri']) {
        $Response.BaseResponse.ResponseUri
    } else {
        $Response.BaseResponse.RequestMessage.RequestUri
    }
    if ($null -eq $responseUri -or $responseUri.Scheme -ne 'https') {
        throw "$Label redirected away from HTTPS"
    }
}

$bundleRoot = [System.IO.Path]::GetFullPath($BundleDirectory)
$recordPath = Join-Path $bundleRoot 'acceptance.json'
if (-not (Test-Path -LiteralPath $recordPath -PathType Leaf)) { throw 'Acceptance record is missing' }
$record = Get-Content -LiteralPath $recordPath -Raw | ConvertFrom-Json
$hostingRoot = Join-Path $bundleRoot 'hosting'
$localManifest = Get-Content -LiteralPath (Join-Path $hostingRoot 'latest.json') -Raw | ConvertFrom-Json
$installerName = [System.IO.Path]::GetFileName(([Uri]$record.installerUrl).AbsolutePath)
$localInstaller = Join-Path $hostingRoot $installerName
if (-not (Test-Path -LiteralPath $localInstaller -PathType Leaf)) { throw 'Local acceptance installer is missing' }
$localSignature = (Get-Content -LiteralPath ($localInstaller + '.sig') -Raw).Trim()
if ($localManifest.version -ne $record.newVersion -or
    $localManifest.platforms.'windows-x86_64'.url -ne $record.installerUrl -or
    $localManifest.platforms.'windows-x86_64'.signature -ne $localSignature) {
    throw 'Local acceptance manifest does not match the recorded installer and signature'
}

$manifestResponse = Invoke-WebRequest -Uri $record.manifestUrl -UseBasicParsing
Assert-HttpsResponse $manifestResponse 'Manifest'
$remoteManifest = $manifestResponse.Content | ConvertFrom-Json
if ($remoteManifest.version -ne $record.newVersion) { throw 'Hosted manifest has the wrong target version' }
$remotePlatform = $remoteManifest.platforms.'windows-x86_64'
if ($remotePlatform.url -ne $record.installerUrl) { throw 'Hosted manifest points to the wrong installer URL' }
if ($remotePlatform.signature -ne $localManifest.platforms.'windows-x86_64'.signature) {
    throw 'Hosted manifest has a different updater signature'
}
$signatureResponse = Invoke-WebRequest -Uri ($record.installerUrl + '.sig') -UseBasicParsing
Assert-HttpsResponse $signatureResponse 'Signature'
if ($signatureResponse.Content.Trim() -ne $localSignature) { throw 'Hosted .sig differs from the verified local signature' }

$downloadPath = Join-Path $bundleRoot ('endpoint-download-' + $PID + '.exe')
if (Test-Path -LiteralPath $downloadPath) { throw "Temporary download path already exists: $downloadPath" }
try {
    $installerResponse = Invoke-WebRequest -Uri $record.installerUrl -UseBasicParsing -OutFile $downloadPath -PassThru
    Assert-HttpsResponse $installerResponse 'Installer'
    $downloadHash = (Get-FileHash -LiteralPath $downloadPath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($downloadHash -ne $record.newInstallerSha256) { throw 'Hosted installer SHA-256 does not match the verified local artifact' }
}
finally {
    if (Test-Path -LiteralPath $downloadPath) { Remove-Item -LiteralPath $downloadPath -Force }
}

Write-Host 'HTTPS acceptance endpoint passed: manifest, signature, installer URL, and installer SHA-256 match.' -ForegroundColor Green
Write-Host 'The app update itself still needs to be run from the old-version installation in a disposable Windows account or VM.'
