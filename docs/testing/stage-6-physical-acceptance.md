# Stage 6.8 Windows and osu! physical acceptance

Run this checklist on a Windows machine with Beatforge, osu! stable and/or osu!lazer installed. Use a short audio file and a disposable generated beatmap. Record `Pass`, `Fail`, or `N/A` plus notes for every applicable row.

## Test record

| Field | Value |
| --- | --- |
| Date | 2026-10-13 |
| Beatforge commit/build | `ba79046` |
| Windows version | |
| osu! stable version | N/A — not installed in this acceptance environment |
| osu!lazer version | Installed version used for physical acceptance; exact version not recorded |
| Installation type | Existing local osu!lazer installation |

## A. Validated result and folder action

- [x] Generate a production `.osz` and wait until the job is `completed`.
- [x] Confirm the job shows `valid`, `valid with warnings`, or `repaired`, plus difficulty/object/BPM/duration statistics.
- [x] Click **Show validated output**. File Explorer should select the delivered `.osz` under `jobs/<job-id>/validated/`.
- [x] Confirm `jobs/<job-id>/raw/` still contains the original candidate and is a different path from the delivered file.
- [x] Restart Beatforge. Confirm the completed state, validation summary, statistics, output path, and both action buttons remain available.

## B. Manual import into osu! stable

N/A for this acceptance run because osu!stable was not installed.

- [ ] In Settings, select the detected stable installation or browse to the folder containing `osu!.exe`, then save.
- [ ] Disable automatic import and generate or reuse a completed `.osz` job.
- [ ] Click **Import into osu!**. Confirm osu! stable starts or receives focus and begins importing the package.
- [ ] Confirm the beatmap appears in the song list and can be opened in the Editor.
- [ ] Confirm osu! may consume the temporary `cache/osu-import` handoff while the original `jobs/<job-id>/validated/` artifact remains available for repeated import.
- [ ] In the Editor, verify audio playback, timing alignment, hit objects, seeking, difficulty metadata, and any packaged background or samples.
- [ ] Confirm osu! does not report a corrupt archive, missing audio, unsupported mode, or unreadable beatmap.

## C. Manual import into osu!lazer

- [x] Select a detected lazer installation that exposes `osu!.exe`, then save.
- [x] Click **Import into osu!** for a completed `.osz` job.
- [x] Confirm lazer starts or receives focus, imports the package, and lists every packaged difficulty.
- [x] Open the imported result in the lazer Editor and verify audio, timing, objects, metadata, background, and samples.
- [x] Record any behavioral difference from stable, including duplicate-import prompts or conversion warnings.

## D. Automatic import

- [x] Select a valid installation folder, enable **Import into osu! automatically**, and save Settings.
- [x] Generate a new valid `.osz`. Confirm osu! is launched exactly once only after validation completes.
- [x] Confirm a `validation_failed`, failed, cancelled, or interrupted job never launches osu!.
- [x] Disable automatic import and generate another result. Confirm osu! is not launched automatically and manual import still works.

## E. Error and safety behavior

- [x] Configure a folder that does not contain `osu!.exe`. Manual import should show an understandable error and must not start another executable.
- [x] Clear the configured path. If no executable is detected but Windows has an `.osz` association, confirm manual import uses that association. Automatic import must not use this fallback.
- [x] Rename or move a disposable validated output, then click either action. Beatforge should report that the artifact cannot be resolved and should not launch osu! or Explorer with another path.
- [x] Confirm jobs without a validated output do not display the normal folder/import actions. A `validation_failed` job with an invalid report may display only **Import failed output anyway**.
- [x] Click **Import failed output anyway**, cancel the warning, and confirm nothing launches. Repeat and accept the warning; confirm osu! receives a disposable handoff while the original report-selected `raw` artifact remains unchanged.
- [x] Confirm the normal actions select/open only `jobs/<job-id>/validated/`. Confirm the failed-output exception resolves only the same job's report-selected `raw/<filename>`, never `working/` or an arbitrary external path.

## Acceptance result

| Target | Result | Notes |
| --- | --- | --- |
| File Explorer action | Pass | Confirmed as part of the applicable Windows acceptance flow. |
| osu! stable manual import | N/A | osu!stable was not installed in the acceptance environment. |
| osu!lazer manual import | Pass | `.osz` imported successfully and opened in the Editor. |
| Automatic import | Pass | Applicable import workflow accepted by the user. |
| Restart persistence | Pass | Applicable persistence workflow accepted by the user. |
| Error/safety cases | Pass | Applicable validation and failed-output import behavior accepted by the user. |

Acceptance was confirmed by the user on 2026-10-13. Imported beatmaps were tested across multiple songs; osu!lazer Editor loading, audio playback, timing behavior, and `.osz` import passed. The managed import handoff preserves the original `raw` or `validated` artifact when lazer consumes its import copy.

Stage 6.8 is accepted when every applicable target passes, imported beatmaps open in the Editor without missing required assets, and no non-validated artifact can reach an operating-system launch action.
