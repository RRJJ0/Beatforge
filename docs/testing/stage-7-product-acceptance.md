# Stage 7 product integration acceptance

This record closes the MVP product-integration stage. It combines automated migration and safety coverage with a Windows desktop smoke test against the existing application profile. Stage 6 gameplay and osu!lazer import evidence remains the source of truth for generated-map playability.

## Test record

| Field | Value |
| --- | --- |
| Date | 2026-10-13 |
| Base commit | `8847c59` plus the stage 7.8 working tree |
| Platform | Windows 11 (26300) |
| GPU | NVIDIA GeForce RTX 4070 |
| Existing profile | 19 persisted jobs, saved settings, managed runtime/model |
| osu! target | Existing osu!lazer installation |

## Automated gates

The following gates passed on the acceptance machine:

```powershell
cd app
npm run test:unit -- --run
npm run type-check
npm run lint
npm run build
npm run tauri -- build --no-bundle

cd src-tauri
cargo test --lib -- --test-threads=1
cargo clippy --lib --tests -- -D warnings
```

Results:

- Vue/Vitest: 5 files and 13 tests passed.
- Rust: 111 tests passed, including jobs/settings migrations, restart recovery, storage cleanup, missing artifacts, validation, preview, osu! handoff, runtime/model recovery, disk reserve, checksum failure, and resumable download.
- Rust Clippy passed with warnings denied.
- Production frontend and the Tauri release executable built successfully.
- The resumable-download fixture explicitly bypasses the user's Windows proxy for its localhost server and joins the server before asserting the client result, preventing environment-dependent connection resets.

## Migration and recovery matrix

| Scenario | Result | Evidence |
| --- | --- | --- |
| Legacy unversioned settings | Pass | Migrates precision/repair defaults and writes the current document. |
| Settings schema v1/v2 to v3 | Pass | Adds notification and locale defaults; export/import round-trip remains valid. |
| Newer or unknown settings | Pass | Rejected before saved settings are modified. |
| Jobs schema v1 to current | Pass | Additive migration is idempotent. |
| Shared migration marker without jobs validation column | Pass | Column is repaired from schema inspection instead of trusting the marker. |
| Restart with queued/running/validating job | Pass | Previous active work becomes `interrupted`; terminal history remains queryable. |
| Restart with completed jobs | Pass | Existing profile restored 19 records with validation and provenance data. |
| Validated artifact removed outside Beatforge or by cleanup | Pass | History is preserved, `outputAvailable` becomes false, a missing-result message is shown, and preview/reveal/import actions are withheld. |
| Disk reserve failure | Pass | Download is rejected before consuming the 64 MiB safety reserve. |
| Cleanup preview and execution | Pass | Preview totals match removal; active jobs, links, models, runtime, and unselected/protected data remain protected. |

## Windows desktop smoke test

- [x] Launch the release executable without a terminal-facing workflow.
- [x] Restore the saved Traditional Chinese locale, settings, detected GPU, runtime status, and notification permission.
- [x] Load persisted History from SQLite and search-ready filters without modifying the records.
- [x] Open a real completed result preview; difficulty, 905-object density, 3:18 duration, and audio waveform load.
- [x] Distinguish seven currently available deliverables from completed historical records whose physical result was removed.
- [x] Hide preview, reveal, and import actions for missing result files while keeping regeneration and history deletion available.
- [x] Preserve validated artifacts when osu!lazer consumes an import handoff, as accepted in stage 6.8.
- [x] Complete the previously accepted production flow across generation, background completion, validation, history, preview, and osu!lazer import using the same persisted profile.

No new production generation was started during this smoke test. Existing production jobs and the stage 6.8 multi-song generation/import acceptance were used so the test did not spend GPU time or create additional user data.

## Known limits and stage 8/9 follow-up

- osu!stable remains untested on this machine; osu!lazer is the physically accepted import target.
- Windows toast delivery still needs a packaged/installed build check. In-app completion/failure notices and notification de-duplication are covered automatically.
- Narrator wording, complete dialog focus trapping, and 200% Windows display scaling need dedicated manual accessibility passes on the packaged build.
- Disk-full behavior is deterministically unit-tested; a destructive physical fill-disk test is intentionally not performed.
- Stage 8 changes the core artifact model for multi-difficulty sets and media packaging, so its work must preserve the migration, validation, storage, preview, and import guarantees recorded here.
- Stage 9 must add installer/signing/update coverage, long-running queue and cancellation soak tests, a clean-machine install, additional NVIDIA GPU/driver coverage, third-party notices, and privacy-safe diagnostic export.

Stage 7 is accepted when the automated gates remain green, the Windows smoke test can restore and operate the existing profile without a command line, and missing or cleaned artifacts never appear as usable deliverables.
