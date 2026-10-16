# Isolated HTTPS updater acceptance

[繁體中文](update-acceptance.zh-TW.md) · [Release candidate runbook](release-candidate-runbook.md)

This test uses `https://localhost` in a disposable Windows account or VM. No public host or GitHub Release is needed. It exercises a real HTTPS download and the updater signature with two distinct SemVer versions, while leaving the checked-in production endpoint and embedded public key unchanged. The test build shares Beatforge's normal application ID and data location: **never install it in your everyday account**.

1. On the development machine, use a verified `0.1.0` candidate artifact to prepare the bundle. This builds a `0.0.9` installer with a test-only updater endpoint. The older installer is Authenticode-unsigned; the newer candidate must have a valid Tauri updater signature. Choose a free local port if `48765` is in use.

   ```powershell
   ./scripts/New-UpdateAcceptanceBundle.ps1 `
     -NewArtifactDirectory './artifacts/windows/Beatforge-0.1.0-<commit>' `
     -ManifestUrl 'https://localhost:48765/latest.json' `
     -InstallerUrl 'https://localhost:48765/Beatforge_0.1.0_x64-setup.exe'
   ```

2. Copy the entire generated `artifacts/update-acceptance/<timestamp>/` directory to a disposable Windows account or VM with Node.js installed. In its first terminal, run the bundled `Start-UpdateAcceptanceServer.ps1` with `-DisposableEnvironment` and leave it open. The script refuses to start if that account already has a Beatforge install or app data. It creates a two-day localhost certificate, temporarily trusts it in **CurrentUser** only, and serves exactly the installer, `.sig`, and `latest.json` on IPv4/IPv6 loopback. It removes the certificate and temporary private-key file when stopped normally with Ctrl+C.

   ```powershell
   ./Start-UpdateAcceptanceServer.ps1 -BundleDirectory . -DisposableEnvironment -Port 48765
   ```

3. In a second terminal in the same test account, confirm the real HTTPS endpoint matches the verified local files. This does **not** perform an app update.

   ```powershell
   ./Test-UpdateAcceptanceEndpoint.ps1 -BundleDirectory .
   ```

4. Install the installer in `old/`. In Settings → Updates, check for `0.1.0` and choose Install and restart. Check the installed executable's **ProductVersion** before (`0.0.9`) and after (`0.1.0`) with `(Get-Item "$env:LOCALAPPDATA\Beatforge\Beatforge.exe").VersionInfo.ProductVersion`; the current sidebar version label is static and is not evidence. Confirm the app starts and a test job or user-data sentinel remains. Keep `old/`, `hosting/latest.json`, `original-candidate-latest.json`, an app-data backup, and the outcome as evidence.
5. Stop the first terminal with Ctrl+C. Check that the localhost certificate thumbprint printed by the server is absent from CurrentUser Root and My stores. If the process was force-killed, remove **only that exact thumbprint** from those two stores manually in the disposable account, and delete the temporary `beatforge-update-acceptance-*.pfx` file reported under its TEMP directory. Record the test account/VM, source and candidate commits, SHA-256 from `acceptance.json`, time, screenshots/logs, and pass/fail in the [release candidate runbook](release-candidate-runbook.md).

The server binds only to `127.0.0.1` and `::1`; it cannot be used from another machine. A successful endpoint check alone is **not** successful updater acceptance. Do not upload the test bundle to the production GitHub Release.
