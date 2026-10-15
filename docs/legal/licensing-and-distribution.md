# Licensing and distribution policy

## Release boundary

Beatforge is licensed under MIT. Third-party components retain their own
licenses. A release artifact may contain only the desktop executable, compiled
frontend assets, icons, `LICENSE`, `THIRD_PARTY_NOTICES.md`, and the inventory
generated from the same commit.

The Python runtime, Mapperatorinator checkout, slider package, PyTorch,
torchaudio, imageio-ffmpeg/FFmpeg and Mapperatorinator model weights are
**download-only components**. They must remain outside the installer until a
separate human review explicitly approves redistribution and records the
applicable source, notices, license text and (where required) source offer.

The model repository declares MIT, but its short model card only names two
training datasets and does not document enough provenance for Beatforge to
make an independent redistribution claim. The current policy is therefore to
fetch the exact pinned revision directly from the upstream repository and
never mirror it as a Beatforge release asset.

imageio-ffmpeg wheels include an FFmpeg executable. FFmpeg is normally LGPL
2.1-or-later, but optional build features can make a binary GPL-licensed. Each
release candidate must capture the installed executable's `ffmpeg -version`
configuration and a reviewer must confirm the resulting obligations. The
package name alone is not sufficient evidence.

## Reproducible inventory

Run this from the repository root after changing a lockfile or dependency:

```powershell
./scripts/Update-ThirdPartyInventory.ps1
./scripts/Test-ThirdPartyLicenses.ps1
```

The generator uses Cargo metadata, pnpm's production license report, the
hash-pinned Python requirements file, and the reviewed Python license mapping.
It removes machine-specific paths and timestamps, sorts every collection, and
writes deterministic JSON. The test regenerates into a temporary path and
requires byte-for-byte equality with the committed inventory.

`Cargo.lock`, `pnpm-lock.yaml`, the Python requirements file, the
Mapperatorinator lock manifest and the generated inventory together identify
the dependency set. `THIRD_PARTY_NOTICES.md` explains the human-reviewed
distribution boundary and is bundled with the application.

## Release blockers

- A dependency is missing a declared license or the reviewed Python mapping.
- A lockfile changed without regenerating the inventory.
- An artifact omits the project license, notices or inventory.
- A download-only component, model, song, beatmap, background or video is
  present in the installer.
- Mapperatorinator, slider or model pins differ from the reviewed manifest.
- The exact imageio-ffmpeg/FFmpeg build has not been manually reviewed.
- A license requires source, relinking material, attribution or another notice
  that the release does not provide.

Automation catches drift and obvious omissions. The release owner must still
review the inventory and effective license terms before publishing.

## Network and privacy boundary

Beatforge does not upload songs, beatmaps, images, videos or generated output
to a Beatforge service. First-time setup does contact the upstream hosts listed
in the inventory to download the Python runtime, Python packages,
Mapperatorinator source and model weights; those hosts can receive ordinary
request metadata such as IP address and user agent under their own policies.
The committed inventory contains no local filesystem paths, usernames or
timestamps. Diagnostic exports use a separate allowlist and do not include
media or beatmap contents by default.

## Public-facing policy

Beatforge-generated beatmaps must disclose Mapperatorinator/AI assistance.
Treat uploads as WIP while obtaining feedback, and check the current osu!
ranking criteria before attempting any status change. Users remain responsible
for rights to every song, image, video, hitsound and other input they provide.
Beatforge is not an official osu! product and receives no osu! endorsement.
