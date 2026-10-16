# Beatforge 0.1.0 release candidate notes

[繁體中文](release-notes.zh-TW.md) · [Home](../../README.md)

Status: release candidate. Publication requires release-gate, updater-signature
and clean-machine acceptance. The publisher must identify which Windows
signing mode was used for the downloadable artifact.

Windows Authenticode unsigned: if the GitHub release uses the free unsigned
distribution path, Windows will not show a verified Publisher and may warn or
block the download or installer under local security policy. The Tauri updater
signature protects update artifacts separately; it does not make the Windows
executable Authenticode-signed. Verify the release source and SHA-256 checksum
before installation. Do not disable Windows security features for Beatforge.

## Included

- Windows x64 current-user NSIS installer and Tauri updater metadata.
- Pinned Mapperatorinator V32 generation, multi-difficulty jobs, validation,
  preview and osu! import.
- Optional background image/video and per-difficulty generation settings.
- Managed runtime/model downloads, offline reuse, diagnostic export and
  English/Traditional Chinese interface.

## Requirements and boundaries

Windows 10/11 x64 is the supported OS target. The first setup downloads the
pinned runtime and model; generation needs adequate local disk and memory.
CUDA compatibility and actual tested hardware are described in
`GPU_SUPPORT_MATRIX.md`. Windows 7, ARM64 and 32-bit Windows are unsupported.

Generated beatmaps require human review. Do not assume that validation or
successful import makes a beatmap ready for ranked submission. Follow the
[AI disclosure and osu! upload guidance](../user/sharing.md).

See `KNOWN_ISSUES.md` and `RELEASE_CANDIDATE_RUNBOOK.md` before testing. The
exact source revision, tool versions and hashes are in `build-provenance.json`.
