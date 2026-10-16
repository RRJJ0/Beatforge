# Beatforge

[繁體中文](README.zh-TW.md)

Beatforge is a local-first Windows desktop app that uses Mapperatorinator to
create **draft osu!standard beatmaps** from your music. It helps with setup,
generation, validation, preview, and import into osu!. It is not an osu! editor,
an automatic uploader, or a guarantee of a playable or rankable map.

## Get started

1. When a release is available, download the Windows x64 installer and its
   `SHA256SUMS.txt` from this repository's [Releases](https://github.com/RRJJ0/Beatforge/releases).
   Check the [system requirements](docs/user/requirements.md) and [known
   issues](docs/release/known-issues.md) first.
2. Install and open Beatforge. The first setup downloads the pinned AI runtime
   and model, so allow time, disk space, and an internet connection. The model
   is not bundled with the installer.
3. Open **Models** to check or repair the runtime and model. Open **Settings**
   to choose the processing device, osu! location, output preferences, and
   language if needed.
4. In **Generate**, choose an audio file, enter the song title and artist, add
   one or more difficulties, then select **Generate**. A background image,
   video, or same-song `.osu` reference is optional.
5. Follow the task in **Jobs**. Review the validation report and preview, then
   import the result into osu!. Use **History** to find earlier jobs. Inspect
   audio, timing, objects, and difficulty in the osu! editor before sharing.

The [full user guide](docs/user/guide.md) explains the controls, failed
validation, import behavior, troubleshooting, and diagnostic export.

## Before downloading or sharing

- Windows 10/11 x64 is the current target. Mapperatorinator V32 for
  osu!standard is the currently supported generation model. See the
  [requirements and GPU support summary](docs/user/requirements.md).
- Public builds may be **Windows Authenticode unsigned**. In that case Windows
  has no verified Publisher for the installer and may warn or block it under
  local security policy. GitHub hosting does not change that. Check the
  release's source revision and SHA-256 checksum; do not disable Windows
  security features for Beatforge. The separate updater signature protects
  downloaded updates, but is not a Windows Publisher signature.
- Generation and validation do not replace human review. If you share a map,
  disclose Mapperatorinator/AI assistance, check the current osu! rules, and
  make sure you have rights to the music and media. Read the
  [sharing and upload guidance](docs/user/sharing.md).
- Beatforge does not upload your songs or maps to a Beatforge service. Initial
  setup contacts upstream download hosts. Review a diagnostic ZIP before
  sharing it. See the [privacy and licensing details](docs/legal/licensing-and-distribution.md).

## More information

- [Release notes](docs/release/release-notes.md) · [Known issues](docs/release/known-issues.md)
- [User guide](docs/user/guide.md) · [System requirements](docs/user/requirements.md)
- [Developer documentation and project history](docs/development/README.md)
- [MIT license](LICENSE) · [Third-party notices](THIRD_PARTY_NOTICES.md)

Beatforge is an independent project and is not affiliated with or endorsed by
osu!.
