# Updates, downgrade protection and offline recovery

## Trust boundary

Beatforge uses Tauri's updater with signed-version enforcement. Every Windows
release produces an NSIS installer plus its updater signature and a static
`latest.json`. The manifest is fetched over HTTPS, but HTTPS and the manifest
alone are not trusted: the downloaded installer must pass the embedded updater
public-key signature before Tauri launches it. The private updater key is never
committed.

Updater signing is separate from Windows Authenticode. Local smoke builds can
use an ignored development updater key. Public builds, including a disclosed
unsigned Windows installer, require an explicitly configured and securely
retained updater private key matching the embedded public key. A CA-issued
Windows Authenticode certificate is optional for the unsigned-public path;
Windows will not display a verified Publisher for that path.

Generate a replacement development key only before any public updater release:

```powershell
cd app
pnpm tauri signer generate --ci -w src-tauri/.release/beatforge-updater.key
```

Copy the generated public-key file contents into `plugins.updater.pubkey` and
store the private key somewhere backed up and access-controlled. Once a public
build ships, rotating or losing this key requires an explicit migration plan.

## Update flow and interruption recovery

The Settings screen can check the static GitHub release manifest. Only a
strictly newer semantic version is accepted; equal versions and downgrades are
rejected. Before applying an update, Beatforge atomically writes
`update-state.json` under its app-data directory with the source version,
target version and attempt time. On the next start it records completion when
the target version is active, or a retryable failure when the old version is
still running.

Windows installs updates in passive current-user mode. A failed or interrupted
installer therefore leaves the previous installation and all app data
available. `latest.json`, the `.sig`, installer checksums and build provenance
are emitted together by `scripts/Build-WindowsInstaller.ps1`.

## Downgrade and migration boundary

The NSIS installer disallows silent downgrades, and the updater accepts only a
version newer than the running app. The database independently checks the
highest `schema_migrations` version before running any migration or manifest
sync. If a user manually installs an older app over newer data, startup stops
with a message that identifies both schema versions and states that the data was
not changed.

Settings documents and managed package manifests already reject schema versions
newer than the app understands. Migrations are additive and idempotent; runtime,
models, jobs, generated output and osu! handoff caches live outside the
application directory and are not removed by an app update.

## Offline and repair behavior

Update checks are optional. A network failure is reported as `offline`, not as
an application startup failure. Beatforge continues to use its built-in pinned
package manifest and already installed, verified runtime and model files. No
network request is made merely to open the app or start generation with healthy
installed components.

Package downloads use `.part` files, resume from a validated byte range and
verify declared size plus SHA-256 before activation. Runtime and model installs
use staging directories and preserve the previous healthy directory until the
replacement passes verification. On restart, stale staging directories are
removed and an interrupted backup is restored. A failed package remains
retryable from the Models screen; a valid cached download is reused without a
network request, while a missing or corrupt artifact is downloaded again.

## Acceptance

Run the GPU-free gate and build an updater-signed unsigned Windows artifact:

```powershell
./scripts/Test-Release.ps1
./scripts/Build-WindowsInstaller.ps1 -AllowUnsigned
```

For lifecycle testing, publish two non-public test versions to an isolated HTTPS
endpoint or draft GitHub release. Verify normal update, an interrupted download,
a modified installer/signature rejection, offline startup, future-schema
rejection without writes, and repair of a damaged runtime/model. Repeat the
public release path during Stage 9.8, recording whether Windows Authenticode
was signed or explicitly unsigned.
