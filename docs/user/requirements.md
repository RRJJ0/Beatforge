# System requirements and GPU support

[繁體中文](requirements.zh-TW.md) · [Home](../../README.md)

Beatforge currently targets **Windows 10/11 x64** and generates
**osu!standard** with the pinned Mapperatorinator V32 model. Windows 7,
Windows ARM64, and 32-bit Windows are not supported. Windows 11 has physical
test records; Windows 10 is a target but has not yet had a dedicated physical
acceptance run.

Initial setup needs internet access, enough free disk space for the managed
Python runtime and model, and possibly WebView2 bootstrap installation. The
installer does not contain the model weights. An already verified local
runtime/model can be reused offline.

| CUDA tier | NVIDIA driver | GPU capability / memory | What to expect |
| --- | --- | --- | --- |
| Recommended | 560.76+ | Compute capability 8.0+, at least 8 GiB VRAM | Primary BF16 target; RTX 4070 physically tested. |
| Minimum | 560.76+ | Compute capability 7.5+, at least 2 GiB VRAM | Can run; older GPUs may use slower FP32. MX450 2 GiB physically tested. |
| Unsupported for CUDA | Older driver, capability below 7.5, or VRAM below 2 GiB | — | CUDA setup and jobs are blocked; CPU mode remains available for slow testing. |

Beatforge temporarily blocks CUDA work if **less than 1536 MiB VRAM is free**;
close other GPU-heavy applications and try again. Meeting the tier thresholds
does not guarantee every song or multi-difficulty job will fit in memory.
NVIDIA 3–7 GiB GPUs, Blackwell, hybrid/multi-GPU arrangements and Windows 10
have not been separately physically accepted. AMD/Intel GPUs are not CUDA
targets. CPU mode is not a performance target.

The **Models** page shows the detected GPU, driver, VRAM, tier, BF16 status,
and setup readiness. The full [hardware evidence and limitations](../testing/stage-9-gpu-support-matrix.md)
are in developer documentation.
