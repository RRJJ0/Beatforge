[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$ArtifactDirectory,
    [switch]$DisposableEnvironment,
    [switch]$AllowUnsigned,
    [int]$StartupSeconds = 5,
    [string]$ResultPath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if (-not $DisposableEnvironment) {
    throw 'This test installs, launches, reinstalls, and uninstalls Beatforge. Run it only in a disposable Windows account or VM and pass -DisposableEnvironment.'
}
if ($StartupSeconds -lt 2 -or $StartupSeconds -gt 60) { throw 'StartupSeconds must be between 2 and 60' }

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$artifactRoot = [System.IO.Path]::GetFullPath($ArtifactDirectory)
$installRoot = Join-Path $env:LOCALAPPDATA 'Beatforge'
$dataRoot = Join-Path $env:APPDATA 'com.beatforge.desktop'
$sentinel = Join-Path $dataRoot 'installer-lifecycle-sentinel.txt'
if ([string]::IsNullOrWhiteSpace($ResultPath)) {
    $ResultPath = Join-Path $env:TEMP 'beatforge-installer-lifecycle.json'
}
$ResultPath = [System.IO.Path]::GetFullPath($ResultPath)

if (Test-Path -LiteralPath $installRoot) { throw "Existing Beatforge installation detected: $installRoot" }
if (Test-Path -LiteralPath $dataRoot) { throw "Existing Beatforge user data detected: $dataRoot" }

& (Join-Path $PSScriptRoot 'Test-WindowsInstaller.ps1') -ArtifactDirectory $artifactRoot -RequireSignature:(-not $AllowUnsigned)
if ($LASTEXITCODE -ne 0) { throw 'Installer artifact verification failed' }
$installer = Get-ChildItem -LiteralPath $artifactRoot -File -Filter '*-setup.exe' | Select-Object -First 1

function Invoke-Installer([string]$Path) {
    $process = Start-Process -FilePath $Path -ArgumentList '/S' -PassThru -Wait
    if ($process.ExitCode -ne 0) { throw "Installer returned exit code $($process.ExitCode)" }
}

function Stop-TestApplication([System.Diagnostics.Process]$Process) {
    if ($Process.HasExited) { return }
    $null = $Process.CloseMainWindow()
    if (-not $Process.WaitForExit(5000)) {
        Stop-Process -Id $Process.Id -Force
        $Process.WaitForExit()
    }
}

Invoke-Installer $installer.FullName
$application = Join-Path $installRoot 'Beatforge.exe'
if (-not (Test-Path -LiteralPath $application -PathType Leaf)) { throw "Installed executable is missing: $application" }

$appProcess = Start-Process -FilePath $application -PassThru
Start-Sleep -Seconds $StartupSeconds
if ($appProcess.HasExited -and $appProcess.ExitCode -ne 0) { throw "Installed application exited with code $($appProcess.ExitCode)" }
Stop-TestApplication $appProcess
if (-not (Test-Path -LiteralPath $dataRoot -PathType Container)) { throw 'First launch did not create the managed app-data directory' }

[System.IO.File]::WriteAllText($sentinel, 'preserve-on-uninstall', [System.Text.UTF8Encoding]::new($false))
Invoke-Installer $installer.FullName
if (-not (Test-Path -LiteralPath $sentinel -PathType Leaf)) { throw 'Reinstall removed managed user data' }

$uninstaller = Join-Path $installRoot 'uninstall.exe'
if (-not (Test-Path -LiteralPath $uninstaller -PathType Leaf)) { throw "Uninstaller is missing: $uninstaller" }
$uninstallProcess = Start-Process -FilePath $uninstaller -ArgumentList '/S' -PassThru -Wait
if ($uninstallProcess.ExitCode -ne 0) { throw "Uninstaller returned exit code $($uninstallProcess.ExitCode)" }

for ($attempt = 0; $attempt -lt 20 -and (Test-Path -LiteralPath $application); $attempt++) { Start-Sleep -Milliseconds 250 }
if (Test-Path -LiteralPath $application) { throw 'Uninstall did not remove the application executable' }
if (-not (Test-Path -LiteralPath $sentinel -PathType Leaf)) { throw 'Uninstall removed managed user data without consent' }

$result = [ordered]@{
    schemaVersion = 1
    testedAtUtc = [DateTimeOffset]::UtcNow.ToString('O')
    installerSha256 = (Get-FileHash -LiteralPath $installer.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    signatureRequired = -not $AllowUnsigned
    install = 'passed'
    firstLaunch = 'passed'
    reinstall = 'passed'
    uninstall = 'passed'
    userDataPreserved = $true
}
$resultParent = Split-Path -Parent $ResultPath
if (-not (Test-Path -LiteralPath $resultParent)) { New-Item -ItemType Directory -Path $resultParent -Force | Out-Null }
[System.IO.File]::WriteAllText($ResultPath, ($result | ConvertTo-Json) + "`n", [System.Text.UTF8Encoding]::new($false))
Write-Host "Installer lifecycle passed. Preserved data remains for inspection at: $dataRoot" -ForegroundColor Green
Write-Host "Result: $ResultPath" -ForegroundColor Green
