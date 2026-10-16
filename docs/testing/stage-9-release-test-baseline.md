# Stage 9.1 release test baseline

## Purpose

Stage 9.1 turns the existing frontend, Rust, validation, lifecycle, and Python worker tests into one repeatable GPU-free release gate. Stage 9.2 extends that Rust suite with bounded end-to-end, fault-recovery, and mock-worker soak coverage. It does not claim to validate the Windows installer, updater, NVIDIA hardware, osu! GUI behavior, or long-running production generation. Those remain explicit manual or hardware gates in stages 9.2 and 9.5–9.8.

## Run the baseline

From the repository root in PowerShell:

```powershell
.\scripts\Test-Release.ps1
```

Use `-SkipBuild` only for a local fast iteration. It is not a passing release result.

The script requires the checked-in `pnpm-lock.yaml` and `Cargo.lock`, an existing `app/node_modules` installation, Node/npm, the Rust toolchain, and Python. It intentionally does not download or repair dependencies, update lockfiles, start the production worker, or use a GPU.

## Automated gate

The command runs these checks in order:

1. Release fixture inventory, size, extension, repository-boundary, and personal-path checks.
2. Vue/TypeScript type-check.
3. Read-only oxlint and ESLint checks.
4. Vitest unit tests.
5. Vite production build.
6. `cargo fmt --check`.
7. Clippy with warnings denied and locked dependencies.
8. Rust tests with locked dependencies.
9. Python worker unit tests.

Any failed step stops the run and returns a non-zero exit code. Test temporary files are routed to a unique ignored directory under `app/.release-test-runs/`; the script restores the caller's `TEMP` and `TMP` values and removes only that validated run directory in `finally`.

## Fixture policy

The versioned inventory is `tests/fixtures/release/manifest.json`. Committed fixtures must be project-authored, reviewable, smaller than the configured limit, stored under an allowlisted fixture root, and listed in the manifest. Real songs, user beatmaps, `.osz` archives, personal paths, downloaded model files, and production databases are not permitted.

Binary-like audio, image, video, archive, model, and SQLite inputs are generated inside an isolated test directory by the test that consumes them. They must not be copied from a user's job directory.

## Gates intentionally deferred

| Gate | Stage | Reason |
| --- | --- | --- |
| Full generation with Mapperatorinator weights | 9.2 / 9.7 | Automated flow uses the mock worker; production soak requires a supported NVIDIA GPU and large managed assets |
| Production-worker soak and resource baseline | 9.2 / 9.7 | Automated tests cover bounded process isolation and recovery; real CUDA/system resource behavior requires hardware |
| Privacy-reviewed diagnostic ZIP | 9.3 | Product feature not implemented yet |
| License and redistribution approval | 9.4 | Requires legal/manual review |
| Installer and code-signing verification | 9.5 | Requires release certificate and clean Windows environment |
| Update, downgrade, offline, and repair matrix | 9.6 | Requires multiple signed application versions |
| GPU and driver support matrix | 9.7 | Requires physical hardware |
| Reproducible signed release candidate | 9.8 | Final release pipeline |

## Initial baseline record

Record the command, revision, tool versions, elapsed time, failed step, and whether `-SkipBuild` was used. A run is a release-baseline pass only when every automated step above completes without file modification and the worktree remains clean apart from changes that existed before the command.
