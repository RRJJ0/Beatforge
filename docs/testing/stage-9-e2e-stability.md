# Stage 9.2 end-to-end and stability verification

## Scope

Stage 9.2 adds a repeatable, GPU-free system boundary around the daily generation lifecycle. It verifies that a multi-difficulty request can become a validated beatmap set, survive SQLite persistence, produce a preview, and create unique osu! import handoff copies without consuming the validated artifact.

The tests only use project-authored synthetic bytes in isolated temporary directories. They do not read a user's songs, jobs, settings, runtime, model, or osu! installation.

## Automated coverage

| Behavior | Automated evidence |
| --- | --- |
| Multi-difficulty job, `.osz` validation, statistics, SQLite completion, preview and handoff | `stage9_acceptance::multi_difficulty_job_reaches_preview_and_non_destructive_handoff` |
| Restart reconciliation and retry | `stage9_acceptance::restart_and_retry_reconcile_sqlite_without_reusing_an_output` |
| Worker success and valid UTF-8 JSONL | `mock_worker_process::worker_completes_over_json_lines` |
| Recoverable generation failure and CUDA OOM | `mock_worker_process::worker_faults_have_stable_machine_readable_outcomes` |
| Crash and malformed protocol output | `mock_worker_process::worker_faults_have_stable_machine_readable_outcomes` |
| Cancellation/timeout process termination and reaping | `mock_worker_process::stalled_worker_can_be_terminated_and_reaped` |
| Repeated isolated jobs and temporary output reclamation | `mock_worker_process::sequential_worker_soak_keeps_jobs_isolated` |
| Validation failure and safe retry state | Rust jobs and validation pipeline unit tests included by the release gate |
| Missing result, duplicate-safe handoff and cleanup | Rust jobs, osu integration and storage unit tests included by the release gate |
| Insufficient disk reserve | Rust download/storage unit tests included by the release gate |

Run the complete GPU-free gate from the repository root:

```powershell
.\scripts\Test-Release.ps1
```

Run a longer mock-worker soak independently:

```powershell
.\scripts\Test-Stage9Stability.ps1 -Iterations 100
```

The soak uses one process per synthetic job, verifies that every terminal event belongs to the expected job, checks that outputs stay in their own directories, removes the complete test root, and compares the `app` process count before and after the run. The iteration count is bounded to 1–1000. A mock-stage delay override keeps this developer test fast; production timeouts and production worker behavior are unchanged.

## Manual production-worker soak

Mock coverage cannot establish Mapperatorinator, PyTorch, CUDA, driver, VRAM, real audio, or osu! GUI stability. Before a release candidate, repeat the following on a supported NVIDIA Windows machine with media the tester has permission to use:

1. Record Beatforge revision, worker version, model version, Windows build, GPU, driver, system RAM, free disk and initial managed-cache size.
2. Queue at least ten songs, including two multi-difficulty sets. Include one cancellation during model loading and one during generation.
3. For every job, record status, stable error code, elapsed time, peak VRAM, process working set and output size. Retry one cancelled/failed job after restarting Beatforge.
4. Preview and import at least one successful set twice. Confirm the validated `.osz` remains and the two handoff copies are unique.
5. Remove or move one completed output and confirm History reports it unavailable without selecting another job's artifact.
6. After the queue drains, confirm no worker remains, SQLite has no `running`/`validating` rows, working directories are reclaimed, and GPU/system memory and managed temporary disk usage return near their initial baseline.

Treat monotonic growth across completed jobs, an orphan process, a stuck active database row, cross-job output, overwritten artifacts, or an unexplained terminal state as a release blocker. GPU compatibility claims and exact hardware records belong to Stage 9.7; installer and real osu! GUI acceptance remain Stages 9.5 and 9.8.
