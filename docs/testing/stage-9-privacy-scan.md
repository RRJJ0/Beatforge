# Stage 9.3 repository privacy scan

## Reproducible check

Run from the repository root:

```powershell
.\scripts\Test-RepositoryPrivacy.ps1
```

The script scans every reachable Git revision for personal Windows user paths, the known development username, common AWS/GitHub/OpenAI token shapes, and private-key headers. It also inventories historical paths with song/audio, beatmap/archive, media, database, model-weight, and checkpoint extensions.

The only binary/media allowlist is the Tauri application icon PNG set. The only `.osu` allowlist is the small project-authored text validation fixtures under `app/src-tauri/tests/fixtures/validation`; they are independently size/content governed by the release fixture policy. A historical `C:\Users\You\.beatforge\models` UI placeholder in `ModelsView.vue` is allowed because `You` is literal example text. The developer username has a separate zero-tolerance rule, so this exception cannot conceal the actual local account name.

The privacy scan is part of `scripts/Test-Release.ps1`. Any unallowlisted historical object or sensitive-pattern path fails the release gate without printing matched secret text.

## 2026-10-15 baseline

The initial Stage 9.3 scan covered 44 reachable revisions at baseline commit `7088803`. It found no credential pattern, known developer username, song/audio file, generated beatmap archive, user database, model weight, or checkpoint in history. Twenty extension-matched objects were reviewed: fourteen Tauri application icons and six synthetic validation `.osu` text fixtures. The only Windows-user-path match was the generic `C:\Users\You\.beatforge\models` UI placeholder described above.

This local pattern and object scan is a release guard, not a promise that arbitrary future secret formats are detectable. GitHub secret scanning and a final release-owner review should remain enabled before and after Public Preview.
