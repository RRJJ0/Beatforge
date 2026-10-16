# Developer documentation

[繁體中文](README.zh-TW.md) · [User README](../../README.md)

The original planning-heavy README is preserved as the
[detailed milestone and decision log](roadmap.zh-TW.md) (Traditional Chinese).
An [English milestone overview](roadmap.md) provides a shorter companion.
These are historical development records, not end-user feature promises.

## Source and architecture

- `app/src/`: Vue UI, stores, desktop services, and localization.
- `app/src-tauri/src/`: Rust desktop commands, jobs, storage, installation,
  validation, updates, and diagnostics.
- `app/src-tauri/resources/python/`: Python worker and pinned AI integration.
- [Mapperatorinator integration](../architecture/mapperatorinator-integration.md),
  [worker protocol](../protocol/worker-protocol.md), and
  [output validation](../architecture/output-validation.md).
- [UI wireframes](../ui/README.md).

## Testing and release

- `scripts/Test-Release.ps1`: GPU-free tests, lint, type-check, and policy gates.
- `scripts/Build-ReleaseCandidate.ps1`: signed or explicitly unsigned-public
  Windows candidate from a clean committed revision.
- [Release-candidate runbook](../release/release-candidate-runbook.md),
  [Windows installer](../release/windows-installer.md), and
  [updater/recovery](../release/updates-and-recovery.md).
- [GPU evidence matrix](../testing/stage-9-gpu-support-matrix.md) and
  [licensing policy](../legal/licensing-and-distribution.md).

Deep protocol, architecture, and test records remain in their original
technical language; the user-facing entry points linked by the root README
have English and Traditional Chinese versions.
