# Windows release-candidate runbook

[繁體中文](release-candidate-runbook.zh-TW.md) · [Developer index](../development/README.md)

Do not publish, tag a final version or upload `latest.json` until every
required check below has a recorded pass. A local unsigned/dirty smoke artifact
is never a publishable candidate.

## Build and provenance

1. Review release notes, known issues, third-party notices, FFmpeg licensing
   and the pinned model's redistribution/training provenance.
2. Commit all intended source and use a clean checkout of the exact revision.
3. Configure the long-lived updater private key corresponding to the embedded
   public key. Keep it outside the repository and artifact directory. For a
   signed Windows build, also configure a trusted code-signing certificate and
   timestamp URL; these are optional for the disclosed unsigned-public path.
4. Run `./scripts/Build-ReleaseCandidate.ps1 -AllowUnsignedPublicRelease` for
   the free unsigned-public path, or omit the switch for a CA-signed build.
   `-AllowUnsignedLocalSmoke` is never publishable. Retain test output, Git
   commit and complete artifact directory. Verify updater cryptographic
   signature, `SHA256SUMS.txt`, `build-provenance.json`, and the recorded
   `releaseMode`. For signed builds also verify both Authenticode signatures,
   Publisher and timestamp. Do not modify files after checksums are generated.
5. For an unsigned-public release, prominently state that Windows Authenticode
   signing is absent and a Windows trust warning may occur; do not claim a
   verified Publisher or instruct users to disable protection. Upload the
   installer and its `.sig` under the URL named by `latest.json`. Publish
   `latest.json` only after the installer is reachable and verified. The
   currently configured `/releases/latest/` endpoint requires a published,
   non-prerelease GitHub Release, not a draft or prerelease.

## Clean Windows acceptance record

Record OS build, account/VM, GPU/driver, source commit, artifact checksum,
date, tester, actual results and evidence locations. Run the lifecycle script
in a disposable account/VM, never an account containing real Beatforge data.

| Check | Result / evidence |
| --- | --- |
| Install; record actual Publisher/SmartScreen behavior for signed or unsigned mode | Pending |
| First launch and runtime/model setup | Pending |
| Generate at least one test beatmap | Pending |
| Validation result and warnings are understandable | Pending |
| Preview and osu! import; inspect editor audio/timing | Pending |
| Export diagnostics and inspect for song/private data | Pending |
| Reinstall preserves settings, models, jobs and outputs | Pending |
| Uninstall preserves user data and removes program | Pending |
| Older version to newer updater-signed artifact from real HTTPS endpoint | User reported pass on 2026-10-17: `0.0.9` → `0.1.0` via isolated `https://localhost:48765/latest.json`. Local bundle: `artifacts/update-acceptance/localhost-20261016` (ignored); target installer SHA-256 `561f406708bc1ce383dc353f3c9d54d6e4f5769bb11eb48bb7b5b0b6959cfd7d`. Windows build, test-account details, and screenshots/logs not yet attached. Production GitHub endpoint remains untested until publication. |
| Interrupted update/download and offline recovery | Pending |
| Downgrade/future-schema rejection without data writes | Pending |

For updater acceptance, use two distinct SemVer builds and the actual embedded
public key; changing `latest.json` alone is not an update test. Keep the older
installer, both manifests and an export/backup of app data for recovery.
Use the [isolated HTTPS updater acceptance procedure](update-acceptance.md) to
prepare and verify the two-version test without changing the production endpoint.

## Rollback and repair

If a candidate fails, stop publishing its manifest and preserve the complete
artifact and evidence. Restore the last known-good installer only when its
version is not older than the local data schema; the app intentionally blocks
unsafe downgrades. Never delete `%APPDATA%\com.beatforge.desktop` to make an
old build start. Instead, back up that directory and use a fixed forward
version or a reviewed data migration. Runtime/model failures can be retried
from Models; a verified cache may be reused offline. For installer repair,
rerun the same verified version and confirm history/output preservation.

Record final release approval separately; the build script cannot attest to
GUI behavior, osu! editor quality or public update availability.
