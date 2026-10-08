# Stage 6.8 Windows and osu! physical acceptance

Run this checklist on a Windows machine with Beatforge, osu! stable and/or osu!lazer installed. Use a short audio file and a disposable generated beatmap. Record `Pass`, `Fail`, or `N/A` plus notes for every applicable row.

## Test record

| Field | Value |
| --- | --- |
| Date | |
| Beatforge commit/build | |
| Windows version | |
| osu! stable version | |
| osu!lazer version | |
| Installation type | installer / portable / Microsoft Store / other |

## A. Validated result and folder action

- [ ] Generate a production `.osz` and wait until the job is `completed`.
- [ ] Confirm the job shows `valid`, `valid with warnings`, or `repaired`, plus difficulty/object/BPM/duration statistics.
- [ ] Click **Show validated output**. File Explorer should select the delivered `.osz` under `jobs/<job-id>/validated/`.
- [ ] Confirm `jobs/<job-id>/raw/` still contains the original candidate and is a different path from the delivered file.
- [ ] Restart Beatforge. Confirm the completed state, validation summary, statistics, output path, and both action buttons remain available.

## B. Manual import into osu! stable

- [ ] In Settings, select the detected stable installation or browse to the folder containing `osu!.exe`, then save.
- [ ] Disable automatic import and generate or reuse a completed `.osz` job.
- [ ] Click **Import into osu!**. Confirm osu! stable starts or receives focus and begins importing the package.
- [ ] Confirm the beatmap appears in the song list and can be opened in the Editor.
- [ ] Confirm osu! may consume the temporary `cache/osu-import` handoff while the original `jobs/<job-id>/validated/` artifact remains available for repeated import.
- [ ] In the Editor, verify audio playback, timing alignment, hit objects, seeking, difficulty metadata, and any packaged background or samples.
- [ ] Confirm osu! does not report a corrupt archive, missing audio, unsupported mode, or unreadable beatmap.

## C. Manual import into osu!lazer

- [ ] Select a detected lazer installation that exposes `osu!.exe`, then save.
- [ ] Click **Import into osu!** for a completed `.osz` job.
- [ ] Confirm lazer starts or receives focus, imports the package, and lists every packaged difficulty.
- [ ] Open the imported result in the lazer Editor and verify audio, timing, objects, metadata, background, and samples.
- [ ] Record any behavioral difference from stable, including duplicate-import prompts or conversion warnings.

## D. Automatic import

- [ ] Select a valid installation folder, enable **Import into osu! automatically**, and save Settings.
- [ ] Generate a new valid `.osz`. Confirm osu! is launched exactly once only after validation completes.
- [ ] Confirm a `validation_failed`, failed, cancelled, or interrupted job never launches osu!.
- [ ] Disable automatic import and generate another result. Confirm osu! is not launched automatically and manual import still works.

## E. Error and safety behavior

- [ ] Configure a folder that does not contain `osu!.exe`. Manual import should show an understandable error and must not start another executable.
- [ ] Clear the configured path. If no executable is detected but Windows has an `.osz` association, confirm manual import uses that association. Automatic import must not use this fallback.
- [ ] Rename or move a disposable validated output, then click either action. Beatforge should report that the artifact cannot be resolved and should not launch osu! or Explorer with another path.
- [ ] Confirm jobs without a validated output do not display the folder/import actions.
- [ ] Confirm the selected/opened file is always under `jobs/<job-id>/validated/`, never `raw/`, `working/`, or an arbitrary external directory.

## Acceptance result

| Target | Result | Notes |
| --- | --- | --- |
| File Explorer action | | |
| osu! stable manual import | | |
| osu!lazer manual import | | |
| Automatic import | | |
| Restart persistence | | |
| Error/safety cases | | |

Stage 6.8 is accepted when every applicable target passes, imported beatmaps open in the Editor without missing required assets, and no non-validated artifact can reach an operating-system launch action.
