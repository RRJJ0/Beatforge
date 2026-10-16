# Stage 9 GPU and driver support matrix

This document is the release-facing source of truth for Beatforge 0.1 GPU support. The machine-readable companion is `gpu-support-matrix.json`. A deterministic policy test keeps its thresholds aligned with the Rust preflight implementation.

## Support policy

| Classification | NVIDIA driver | Compute capability | Dedicated VRAM | Behavior |
| --- | ---: | ---: | ---: | --- |
| Recommended | 560.76 or newer | 8.0 or newer | 8 GiB or more | CUDA generation and native BF16; this is the primary release target. |
| Minimum | 560.76 or newer | 7.5 or newer | 2–8 GiB | CUDA generation is allowed. Pre-Ampere GPUs use the slower FP32 fallback; close other GPU applications and prefer fewer difficulties per job. |
| Partially tested | Current driver, but the probe cannot identify VRAM or compute capability | Unknown | Unknown | CUDA may be attempted, but Beatforge does not describe the combination as supported. Export diagnostics when reporting results. |
| Unsupported | Older than 560.76, below compute capability 7.5, or below 2 GiB | — | — | CUDA package installation and job creation are blocked with recovery guidance. CPU mode remains available for slow compatibility testing. |

Regardless of tier, Beatforge temporarily blocks a CUDA job when less than 1536 MiB VRAM is free. This is a recoverable state: close GPU-heavy applications and rerun diagnostics. Driver thresholds follow the CUDA 12.6-or-newer PyTorch wheel baseline pinned by Beatforge; a newer driver may select a newer wheel.

BF16 capability is inferred from NVIDIA compute capability 8.0 or newer and is displayed separately from PyTorch runtime readiness. Mapperatorinator V32 falls back to FP32 on the recorded Turing baseline. The compatibility tier describes tested hardware capacity; `Ready` separately confirms that Python, PyTorch, Mapperatorinator, the model package and CUDA runtime are installed and loadable.

## Physical test records

| Tier / scenario | Windows and hardware | Driver / precision | Install and model load | Peak VRAM | Generation evidence | Stability and limits |
| --- | --- | --- | --- | ---: | --- | --- |
| Recommended / primary development | Windows 11 build 26300; RTX 4070, Ada, compute 8.9, 12282 MiB | 595.97 and 610.88 / native BF16 | Pass / Pass | 1175.2 MiB allocated | Single-difficulty jobs: 154.6–172.5 s. Repeated 2–4 difficulty jobs: 278.5–998.6 s total inference time. | Multiple persisted production jobs completed. Wall time varies with song length and difficulty; allocated VRAM is not driver-reserved VRAM. |
| Minimum and low-VRAM | Windows (exact build was not captured); MX450, Turing, compute 7.5, 2048 MiB | 573.91 / FP32 fallback | Pass / Pass | 1636.4 MiB allocated | Two single-difficulty runs: 221.941 s and 212.201 s. | Reproducible output passed twice. Multi-difficulty was not physically recorded; use fewer difficulties and keep other GPU applications closed. |

The minimum configuration is intentionally also the required low-VRAM scenario, so the two available physical machines cover all three Stage 9.7 acceptance categories: minimum, recommended, and low VRAM. The RTX 4070 timings were read from privacy-local persisted provenance on 2026-10-16; no song, title, artist, path, seed, or beatmap content is copied into this record. The MX450 record is preserved in `stage-5-gpu-acceptance.md`.

## Unverified combinations

- NVIDIA GPUs with 3–7 GiB VRAM satisfy the policy but have not received a dedicated physical acceptance run. They remain minimum-tier capacity, not recommended hardware.
- Blackwell and later architectures are capability-compatible but not physically tested by this project.
- Windows 10, ARM64 Windows, laptop hybrid-GPU switching, eGPU, multi-GPU selection and virtualized GPU configurations are not physically tested.
- Studio versus Game Ready driver variants are not distinguished; the recorded version and CUDA availability are the relevant observations.
- AMD, Intel and Apple GPUs are not supported CUDA targets. CPU mode is not a release-performance target.

## User-visible preflight and recovery

The Models page displays the same tier, architecture, total/free VRAM, BF16 status, summary and recommendations returned by the Rust policy. CUDA backend choices are disabled when hardware preflight fails. The same preflight runs before CUDA dependency installation and before a job record is created, preventing predictable driver, capacity or free-memory failures from reaching inference.

The privacy-safe diagnostic ZIP includes this compatibility result inside `system.json`. Hardware reports should include the diagnostic ID and matrix tier, but must not include songs or beatmaps.

## Acceptance procedure for another GPU

1. Install the current Beatforge build on a supported x64 Windows account and open Models.
2. Record the displayed GPU, architecture, total/free VRAM, driver, tier and BF16 status.
3. Install or repair the managed runtime/model and run both environment diagnostics and worker health.
4. Generate one difficulty, then a set with at least two difficulties. Record only elapsed time, peak allocated VRAM, status and stable error codes.
5. Repeat the single-difficulty job once to check stability. Restart Beatforge and verify the provenance remains available.
6. Export the privacy-safe diagnostic package and update both matrix files only from physical evidence. Automated fixtures may test policy branches but may never be listed as hardware acceptance.

Any model-load failure, CUDA OOM on a tier claimed as supported, silent precision mismatch, repeated worker crash, or disagreement between the UI and this matrix is a release blocker for that combination.
