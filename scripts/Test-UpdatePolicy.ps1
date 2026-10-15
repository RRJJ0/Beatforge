[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$configPath = Join-Path $repoRoot 'app\src-tauri\tauri.conf.json'
$windowsConfigPath = Join-Path $repoRoot 'app\src-tauri\tauri.windows.conf.json'
$config = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
$windows = Get-Content -LiteralPath $windowsConfigPath -Raw | ConvertFrom-Json

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

Assert-True ($config.bundle.createUpdaterArtifacts -eq $true) 'Updater artifact generation is disabled'
Assert-True ($config.plugins.updater.pubkey -match '^[A-Za-z0-9+/=]+$') 'Updater public key is missing or malformed'
Assert-True ($config.plugins.updater.requireSignedVersion -eq $true) 'Update version binding must remain enabled'
Assert-True ($config.plugins.updater.endpoints.Count -gt 0) 'No update endpoint is configured'
foreach ($endpoint in $config.plugins.updater.endpoints) {
    Assert-True ($endpoint -match '^https://') "Update endpoint must use HTTPS: $endpoint"
}
Assert-True ($windows.bundle.windows.allowDowngrades -eq $false) 'NSIS downgrade protection is disabled'

$trackedPrivateKeys = @(& git.exe -C $repoRoot ls-files | Where-Object { $_ -match '(?i)(updater.*\.key$|\.pfx$|\.p12$)' })
Assert-True ($trackedPrivateKeys.Count -eq 0) "Updater/code-signing private key is tracked: $($trackedPrivateKeys -join ', ')"

$databaseSource = Get-Content -LiteralPath (Join-Path $repoRoot 'app\src-tauri\src\database_compatibility.rs') -Raw
Assert-True ($databaseSource.Contains('stopped before changing your data')) 'Future database schema guard is missing'
$updateSource = Get-Content -LiteralPath (Join-Path $repoRoot 'app\src-tauri\src\app_updates.rs') -Raw
Assert-True ($updateSource.Contains('candidate > current')) 'Strict upgrade-only version policy is missing'
Assert-True ($updateSource.Contains('Installed runtime and models remain usable offline')) 'Offline update behavior is not explicit'

Write-Host 'Update, downgrade and offline policy passed.' -ForegroundColor Green
