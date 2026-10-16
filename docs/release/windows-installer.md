# Windows installer and signing

[繁體中文](windows-installer.zh-TW.md) · [Developer index](../development/README.md)

## Supported release shape

Beatforge currently targets 64-bit Windows 10 and Windows 11. The release is a
per-user NSIS installer, so installation does not request administrator access.
It installs under `%LOCALAPPDATA%\Beatforge` and creates a Start Menu entry in
the Beatforge folder. Older versions cannot silently replace a newer version.

The installer downloads Microsoft's WebView2 bootstrapper only when the
required runtime is absent. This keeps the installer small and means initial
installation can require internet access. The Visual C++ runtime DLLs are
bundled. Windows 11 normally already includes WebView2; a missing runtime is
handled by the installer. Windows 7, ARM64 and 32-bit Windows are not supported.

The NVIDIA CUDA Toolkit, Git, Python, FFmpeg and model weights are not installer
prerequisites. Beatforge downloads its pinned Python/AI environment during
first-time setup. An NVIDIA GPU is recommended but the environment check can
select CPU mode. The download and GPU compatibility matrix remains Stage 9.7.

## User-data boundary

Application binaries live in `%LOCALAPPDATA%\Beatforge`. Settings, job history,
logs, generated outputs, runtime and models live separately under
`%APPDATA%\com.beatforge.desktop`, using Tauri's `app_data_dir()`.

Reinstall replaces application files but must preserve the app-data directory.
The standard uninstaller removes the installed program and shortcuts only; it
must not remove the app-data directory. Removing models, history or generated
work is a separate explicit in-app/user filesystem action. This separation also
makes a failed installation safe to retry.

## Local unsigned smoke build

This direct command makes a local smoke artifact, not a public release. It may
trigger Windows trust warnings:

```powershell
./scripts/Build-WindowsInstaller.ps1 -AllowUnsigned
```

The script refuses a dirty worktree unless `-AllowDirty` is explicitly used.
It runs the GPU-free release gate, creates only the NSIS target, and writes a
version/commit-specific directory under `artifacts/windows/`. The directory
contains the installer, standalone executable, legal notices, deterministic
dependency inventory, `SHA256SUMS.txt`, and `build-provenance.json`.

## Public release builds

For an unsigned public candidate, use the explicit switch below from a clean,
committed revision. This does not require a paid Windows certificate, but it
does require an explicitly configured, securely retained Tauri updater private
key matching the embedded public key:

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = '<secure path to updater private key>'
./scripts/Build-ReleaseCandidate.ps1 -AllowUnsignedPublicRelease
```

The build runs the full GPU-free gate and packages release notes, known issues,
support matrix and the acceptance runbook. The artifact provenance records
`releaseMode: unsigned-public`; the notes disclose that Windows Authenticode
is absent. `-AllowUnsignedLocalSmoke` remains explicitly non-publishable.
Windows may warn or block unsigned installers under local policy. GitHub
hosting and the updater signature do not provide a verified Windows Publisher.
The runbook documents the remaining human acceptance and publication order.
Artifact verification uses the Rust toolchain to verify the updater `.sig`
cryptographically against the public key embedded in Tauri configuration and
the declared version; it also checks file hashes and signer provenance.

For a CA-signed public build, omit `-AllowUnsignedPublicRelease` and configure
the certificate and timestamp as follows:

Import a trusted code-signing certificate with an accessible private key into
the Windows certificate store, then set process/CI secrets without writing them
to repository files:

```powershell
$env:BEATFORGE_WINDOWS_CERTIFICATE_THUMBPRINT = '<40 hex SHA-1 thumbprint>'
$env:BEATFORGE_WINDOWS_TIMESTAMP_URL = '<certificate-provider timestamp URL>'
./scripts/Build-ReleaseCandidate.ps1
```

Use `-TimestampUsesRfc3161` when the certificate provider requires RFC 3161
timestamping. The script validates the certificate, private key, expiry and
Code Signing EKU; Tauri signs the executable before bundling and signs the NSIS
installer. It then requires `Get-AuthenticodeSignature` to report `Valid` for
both files. The same signed executable is the payload used when Stage 9.6 adds
updater archives.

Both public modes refuse dirty source, stale notices, test failures and an
existing artifact directory. The signed mode additionally requires valid
Authenticode and timestamping. Checksums and provenance record the full Git
commit, clean/dirty state, tool versions, lockfile hashes, artifact hashes,
release mode and any signer identity. Private keys and certificate passwords
are never recorded.

## Disposable clean-machine acceptance

Use a fresh Windows account, VM or Windows Sandbox with no Beatforge install or
Beatforge app-data directory. Copy the complete artifact directory into it and
run as the ordinary test user:

```powershell
./scripts/Test-WindowsInstallerLifecycle.ps1 `
  -ArtifactDirectory '<artifact directory>' `
  -DisposableEnvironment
```

For an unsigned-public candidate or local smoke artifact, add `-AllowUnsigned`:

```powershell
./scripts/Test-WindowsInstallerLifecycle.ps1 `
  -ArtifactDirectory '<artifact directory>' `
  -DisposableEnvironment `
  -AllowUnsigned
```

The lifecycle test refuses a non-empty profile. It verifies the artifact and
checks silent install, first launch, reinstall, uninstall and preservation of a
sentinel in the managed app-data directory. It intentionally leaves that
directory behind for inspection; remove the disposable account/VM afterward.

Manual checks still required for the release record:

1. Record actual Publisher and SmartScreen behavior. For unsigned-public
   artifacts, do not claim a verified Publisher or recommend disabling Windows
   security features.
2. Confirm Start Menu launch and optional desktop shortcut behavior.
3. On a machine without WebView2, confirm bootstrap installation and retry
   behavior after deliberately interrupting the first attempt.
4. Complete first-time runtime/model setup and verify all large files appear
   under the app-data directory, not the install directory.
5. Generate a small test beatmap, restart, reinstall, and confirm history and
   output remain available.
6. Uninstall and confirm the program/shortcut are gone while runtime, models,
   settings, history and output remain.
7. Verify `SHA256SUMS.txt`, `build-provenance.json` and the updater signature.
   For signed artifacts, also verify both Authenticode signatures and timestamp.

Stage 9.5 established the installer and disposable-machine workflow. Stage 9.8
accepts a clearly disclosed unsigned-public release after all other gates and
clean-machine evidence pass; CA signing remains an optional future improvement.
Keep the lifecycle JSON and manual notes with the release candidate.
