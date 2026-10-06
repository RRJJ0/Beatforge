# Stage 5 GPU acceptance

This acceptance closes the Mapperatorinator integration without treating a generated map as gameplay-valid; `.osu`/`.osz` content validation belongs to stage 6.

## Automated gates

Run from `app` unless noted otherwise:

```powershell
pnpm type-check
pnpm lint
pnpm test:unit -- --run
cd src-tauri
cargo test
cargo clippy --all-targets -- -D warnings
```

The production worker must also compile under the application-managed Python 3.10 interpreter. Unit coverage verifies protocol provenance parsing, SQLite persistence, request metadata, cancellation/process-tree cleanup, and non-UTF-8 stderr draining.

## Reproducibility checks

For two runs with the same audio/reference files and generation settings:

1. `reproduction_key`, input SHA-256 values, source commit, model version, effective parameters, and seed are equal.
2. Each run has its own job id, timestamps, elapsed time, runtime/driver observations, output SHA-256, and output path.
3. Changing the seed, an effective parameter, source/model identity, or input bytes changes `reproduction_key`.
4. The completed job returned to Vue contains the persisted provenance object after an application restart.

## Physical Windows/NVIDIA flow

1. On the Models page, verify the managed runtime/model and worker health are Ready.
2. On Generate, select a local MP3/WAV/OGG/M4A/FLAC, keep scenario `success`, record the seed, and create the job.
3. Confirm stable progress/heartbeat updates and that cancel leaves no Python/CUDA process or job temporary directory.
4. Let a job finish. Confirm the Jobs page shows an `.osz`/`.osu` output plus run key, GPU, effective precision, elapsed time, and peak allocated VRAM.
5. Restart Beatforge and confirm the same provenance is loaded from SQLite.
6. Retry with unchanged input/settings and compare the two records using the reproducibility checks above.

## Recorded development-machine run

| Item | Observation |
| --- | --- |
| GPU | NVIDIA GeForce MX450 |
| Dedicated VRAM | 2048 MiB |
| NVIDIA driver | 573.91 |
| PyTorch / CUDA runtime | 2.10.0+cu128 / 12.8 |
| Source commit | `0e2b0e387aab4b35c64b0b11b12d47578dea7587` |
| Precision | BF16 unsupported; V32 safely resolves to FP32 |
| Test input | 45-second FLAC acceptance clip |
| Run 1 | Completed, 221.941 s, 1636.4 MiB peak allocated VRAM |
| Run 2 | Completed, 212.201 s, 1636.4 MiB peak allocated VRAM |
| Reproduction key | `cb760bf656b2a467ba813643f8218010f8100dcc9ac122bbf03eb9b6a3c3cfc9` for both runs |
| Input SHA-256 | `760ebf65b5ddeed2fa9f2d5ec72bc4bc7191fef61d9d37db9089c26a8b69b680` for both runs |
| Generated `.osu` payload | `9a90196499ff0f6f965a5387f10ba87ef5938d9992e00b8cbb5d4bfa5b40220a` for both runs |
| Result | Two production worker runs completed with full provenance and playable-package candidates for stage 6 validation |

The two outer `.osz` files intentionally have different artifact hashes because upstream assigns a random internal `.osu` filename and the ZIP stores creation timestamps. Their generated `.osu` payload bytes are identical. Reproducibility is therefore represented by the stable reproduction key and content summaries rather than an incorrect promise that independently packaged ZIP bytes are identical.

## Known limits and diagnosed failures

- MVP output is osu!standard only; position diffusion and experimental fast decoder loops are disabled.
- A 2 GiB MX450 is supported through FP32 fallback but is much slower than the recommended GPU class. Peak allocated memory is recorded per successful run; driver-level reserved memory is not the same metric.
- The model and backbone configuration are strictly local/offline. Missing cache files fail as model/configuration errors instead of downloading from Hugging Face during a job.
- V32 FP16 timing sampling produced invalid distributions on this stack; V32 treats the legacy FP16 setting as BF16 and falls back to FP32 when native BF16 is unavailable.
- CUDA device-side assertions poison the process CUDA context. Beatforge reports a stable error and requires a fresh worker process for retry.
- Windows console progress may contain non-UTF-8 bytes. Rust drains stderr as bytes so logging cannot close the pipe and cause Python exit code 120.
