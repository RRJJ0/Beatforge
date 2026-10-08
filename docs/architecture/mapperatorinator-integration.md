# Mapperatorinator Integration Boundary

## Locked upstream identities

Beatforge does not follow either upstream repository's moving default branch at runtime.

| Component | Locked revision | License | Distribution role |
|---|---|---|---|
| Mapperatorinator source | `0e2b0e387aab4b35c64b0b11b12d47578dea7587` | MIT | Installed into the application-managed runtime in phase 5.2 |
| OliBomby slider fork | `ea28c484a0a542d6e094e25aa02e1529ff2c8313` | LGPL-3.0-or-later | Installed from a checksum-pinned HTTPS archive; system Git is not required |
| Mapperatorinator V32 model | `74f22583400d259bf424819e11027c17933efe54` | MIT | Managed model package created in phase 4.5 |
| Beatforge adapter | Current application version | Project license | Owns protocol, validation, progress, cancellation, and error translation |

The machine-readable lock is `app/src-tauri/resources/manifests/mapperatorinator-v32.lock.json`. It records source and slider archive URLs, byte sizes, SHA-256 values and licenses, plus hashes of the inspected inference entry point, upstream requirements, utility re-export, and V32 configuration. Application startup validates its schema, security-sensitive fields, and the embedded dependency lock fingerprint.

Updating the source or model revision is an explicit product change. The update must regenerate all affected hashes, review upstream dependency and configuration changes, run the phase 5 adapter tests, and repeat GPU acceptance before release.

## Adapter strategy

The production worker will import the pinned Python source and call a narrow API surface:

1. `compile_args`
2. `setup_inference_environment`
3. `load_model_with_server`
4. `get_config`
5. `generate`

The adapter owns construction of `InferenceConfig`; Vue and Rust never construct Hydra overrides. The managed V32 osu!standard directory is always supplied as an absolute local `model_path`. Runtime resolution of a Hugging Face repository name is disabled, so inference cannot silently download or switch weights.

The adapter must redirect upstream `print` and logging output to stderr or the local job log. Worker stdout remains exclusively one JSON object per line under the versioned phase 3 protocol.

## Production worker bootstrap

The version 8 bootstrap is embedded in the Rust binary and atomically materialized under `runtime/worker/8` before launch. Rust starts it with the managed Python interpreter in isolated mode and sends exactly one newline-terminated command. Both directions enforce the 64 KiB phase 3 message limit. The bootstrap captures stdout while importing PyTorch, FFmpeg and Mapperatorinator so upstream prints cannot become protocol frames; diagnostic output and tracebacks remain on stderr and are capped before being appended to the managed worker log.

The `health_check` command imports the pinned source without loading model weights, validates the locked callable surface, resolves the managed FFmpeg executable, reports CUDA visibility, and checks the expected V32 model files. Its single `health` response carries protocol and worker versions plus component identity and structured errors. A non-ready health response is valid protocol data rather than a malformed-worker failure.

The phase 3 mock executable remains the default generation backend until phase 5.5 connects inference. This keeps deterministic success, failure, malformed-output, crash, timeout and cancellation tests independent of GPU availability while phase 5.4 validates the production request boundary.

## Production request contract

The Vue domain request selects only the managed package id `mapperatorinator-v32-standard`; it has no checkpoint or output-path field. Rust accepts gamemode 0, difficulty 1.0–8.0, seed 0–65535, bounded positive and negative descriptor lists, optional `.osu` reference context, AR/CS 0–10, and a hitsound switch. Audio extensions follow the pinned inference entry point: MP3, WAV, OGG, M4A, and FLAC.

Before a command is eligible for the production worker, Rust canonicalizes the input files and resolves the source, model, and per-job output from application-managed roots. It rejects traversal outside those roots and checks the source marker plus all four fixed model files. Device and precision come from validated application settings. The serialized command contains only this resolved identity and a whitelist `config` object; mock scenarios and the frontend-facing model selector never cross the production boundary.

The Python bootstrap repeats structural and range validation before importing inference code. A valid production command runs the pinned `compile_args`, `setup_inference_environment`, `load_model_with_server`, `get_config`, and `generate` API surface without changing the domain contract.

## Inference execution

Normal `success` jobs use the application-managed Python production worker. Explicit phase 3 failure simulations remain routed to the mock executable. The production process forces Hugging Face and Transformers offline, selects the managed local checkpoint with gamemode auto-selection disabled, and disables the upstream inference server so cancellation and process ownership stay with the Rust job manager. V32 runs without its optional diffusion position model and exports an `.osz` containing the generated beatmap and source audio.

The worker emits stable stages for adapter validation, runtime import, configuration, model loading, audio preparation, generation, and output validation. Model loading and generation send a JSON Lines heartbeat every 15 seconds. Rust accepts only a final `.osu` or `.osz` resolved beneath the managed per-job output directory. Optional reference beatmaps provide the upstream timing context; arbitrary overwrite and add-to-beatmap modes remain disabled.

The pinned V32 `MapperatorinatorConfig` constructor attempts to resolve `openai/whisper-small` even though the verified model `config.json` already contains the complete VarWhisper backbone configuration. Before loading weights, the production adapter installs a V32-specific compatibility shim that constructs `VarWhisperConfig` from that embedded object. It fails closed unless the backbone identity and expected V32 dimensions match, and it never falls back to a remote repository.

The V32 YAML enables an experimental CUDA-graph decoder. With the locked PyTorch 2.10 environment, its timing pass can produce an all-zero sampling distribution and trigger a device-side assertion in `torch.multinomial`. Beatforge disables both fast decoder flags and uses the upstream standard Transformers generation path until that optimization has a separately pinned compatibility test.

## Failure and process lifecycle

The production bootstrap translates known failures into stable protocol codes for CUDA memory exhaustion, CUDA environment incompatibility, CUDA runtime failure, unavailable model or input files, audio decoding, pinned configuration failure, cancellation, and unknown upstream inference errors. Protocol messages contain a user-facing summary and retryability only; the original exception chain and traceback remain in the per-job local log.

Every successful production result carries provenance schema v1. It records the pinned source commit and model package version, worker/protocol versions, Python/PyTorch/CUDA and detected GPU identity, the complete adapter-effective inference settings including seed, SHA-256/size summaries for inputs and output, UTC timestamps, elapsed time, peak allocated CUDA memory, and known feature limits. A canonical SHA-256 `reproduction_key` covers source/model identity, worker version, effective parameters, and input summaries; reruns with the same inputs and settings therefore share a traceable identity even when stochastic output bytes are compared separately. Rust persists this document in the job table's `result_json` field before publishing completion.

Rust keeps the worker stdin pipe open after `start_job`. Cancellation sends a protocol-versioned `cancel_job` frame for the same job id. The Python watcher raises SIGINT in the inference process so normal stack unwinding gets the first opportunity to release model and audio resources. Rust waits three seconds, then terminates the complete Windows process tree and reaps the child if graceful cancellation did not finish. Timeout, protocol failure, app shutdown, and monitor failure use the same process ownership boundary.

Failed, cancelled, interrupted, and start-failed jobs do not retain their managed `jobs/<id>` working directory. Retry cleans it before requeueing, and startup removes directories belonging to jobs recovered from a previous running or queued state. Cleanup canonicalizes the target and only removes a direct child of the managed jobs root; successful output remains intact. Stderr is drained as raw bytes rather than UTF-8 lines because upstream progress bars can use the active Windows code page. This prevents a decoding error from closing the pipe and turning Python stream finalization into exit code 120.

## Managed Python environment

The application installs source under `runtime/mapperatorinator/<commit>` and keeps the verified archive in its managed cache. Activation uses staging and backup directories so an invalid replacement does not destroy the prior source version. A local marker binds the active tree to the source commit, dependency lock hash, and slider commit.

`resources/python/mapperatorinator-windows-py310.requirements.txt` is a platform-specific, fully hashed lock generated from the reviewed inference-only input with uv 0.12.21. PyTorch and torchaudio are deliberately omitted because phase 4 selects their index from the detected NVIDIA driver; all other emitted packages are installed with dependency resolution and build isolation disabled and hashes required. The slider fork is represented by a commit-specific HTTPS archive rather than a Git URL. A global wheel-only policy is intentionally not used because Hydra pins `antlr4-python3-runtime==4.9.3`, which PyPI publishes only as a source distribution; its archive remains version- and hash-locked.

The pinned source eagerly re-exports `train_utils` from `osuT5.osuT5.utils`, which imports training-only packages even during inference startup. Installation replaces that exact three-line upstream file with an exact two-line inference-only variant that retains `init_utils` and `model_utils`. The operation is guarded by a full-content comparison and fails closed if upstream content changes. This is the only source compatibility patch in phase 5.2.

## MVP scope

Included:

- Windows 10/11 and the application-managed Python 3.10 runtime.
- Mapperatorinator V32 osu!standard generation (`gamemode = 0`).
- Local audio input and, after adapter validation is complete, an optional local `.osu` reference.
- Local managed model files whose SHA-256 matches the phase 4 manifest.
- `.osu` or upstream-exported `.osz` output handed to the phase 6 validator.

Excluded from the phase 5 adapter:

- Upstream Hydra command-line execution.
- Upstream Web UI, Flask server, pywebview, and static assets.
- MaiMod, training, dataset tools, LoRA training, and model publishing.
- Arbitrary checkpoint or LoRA paths supplied by the frontend.
- Remote model resolution during a generation job.
- Taiko, catch, and mania until each receives its own model package and GPU acceptance record.

## Responsibility split

Beatforge validates all external paths and domain parameters before Python starts. It owns process lifetime, job state, timeout, cancellation, persistence, and safe output directories. The Python adapter translates the validated request into Mapperatorinator types, loads the pinned local model, reports stable progress stages, and returns the generated path. Upstream tracebacks are diagnostic data and must never be written to protocol stdout.

Mapperatorinator remains responsible for audio preprocessing, token generation, timing generation, post-processing, and initial file export. Phase 6 remains responsible for treating that output as untrusted, parsing it, validating osu! semantics, and deciding whether it can be presented as a successful result.

## License handling

The pinned Mapperatorinator source and V32 model both declare MIT. The pinned slider fork declares LGPL-3.0-or-later. Any distributed source copy must retain both projects' license and copyright notices. Phase 8 must include these entries in third-party notices together with the independently reviewed licenses of Python dependencies and model-related assets.
