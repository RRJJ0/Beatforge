# Beatforge 0.1.0 release notes

[繁體中文](release-notes.zh-TW.md) · [Home](../../README.md)

Status: released. Beatforge v0.1.0 was published on 2026-10-16 at 17:36 UTC
from source commit `51ef87d06a6b556abf377973574412315447d174`. The release is
the intentionally disclosed `unsigned-public` Windows build: the updater
artifact is signed, while the downloadable Windows executable is not
Authenticode-signed.

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

Before installing, read the [known issues](known-issues.md). The preserved
[release-candidate runbook](release-candidate-runbook.md) documents the build
and acceptance process. The exact source revision, tool versions and hashes
are in the published `build-provenance.json`.
