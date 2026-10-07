# Output validation contract

## Scope

Phase 6 treats every Mapperatorinator result as an untrusted candidate artifact. Python inference ending successfully is not application success: Rust must preserve the original artifact, validate it, optionally create a repaired copy, and persist a versioned report before a job may become `completed`.

The first release accepts a single `.osu` file or an `.osz` archive containing osu!standard beatmaps (`Mode: 0`). Parsing begins in phase 6.2; this document fixes the boundary, serialized report, limits, and artifact ownership used by the later implementations.

## `.osu` parser boundary

The phase 6.2 Rust parser has byte and file entry points. The file entry point verifies that the path is a regular file and enforces the configured size ceiling from filesystem metadata before reading; the byte entry point repeats the size check so in-memory callers have the same boundary. It accepts valid UTF-8 with an optional BOM, LF or CRLF, blank lines, and `//` comments. Invalid encoding, an oversized file, or a missing/unsupported header returns no partial beatmap. All other structural findings are collected in deterministic source order with one-based line context where available.

The current model supports Mapperatorinator's `osu file format v14` and osu!standard `Mode: 0`. `General`, `Metadata`, and `Difficulty` have typed required values while preserving every parsed key-value field. Timing points and hit objects retain their raw line, comma-separated tokens, and source line for phase 6.3 semantics. Unknown valid fields and sections are retained for forward compatibility; duplicate known sections and case-insensitive duplicate fields are rejected as ambiguous. The parser never rewrites the candidate bytes.

## Timing and hit-object semantics

Phase 6.3 consumes the parsed model without reading or rewriting files. It returns typed timing points and hit objects plus ordered issues, so invalid entries can be rejected while the report still describes later independent findings.

Timing validation requires finite, time-ordered values; valid meter/sample/volume/flag fields; the correct beat-length sign for inherited and uninherited points; finite derived BPM or slider-velocity values; and at least one uninherited timing point. The difficulty scalars HP, CS, OD, and AR must remain in 0-10, while slider multiplier and tick rate must be positive.

Hit-object validation accepts osu!standard circles, sliders, and spinners with legal combo and hit-sound flags. Object origins must use integer coordinates inside `0..=512 x 0..=384`, and object start times must be non-negative integers in source order. Sliders require a B/C/L/P curve with parseable `x:y` control points, a positive repeat count, and a finite positive pixel length. A control point outside the visible playfield is a warning because osu! can represent such geometry; an out-of-bounds object origin is an error. Spinner end time must follow start time. When an audio duration is available, an object known to end beyond it receives `OBJECT_AFTER_AUDIO_END` as a warning.

Slider end-time calculation depends on timing/SV state and is deferred to the statistics pass. Until then, the optional audio-duration check uses object start time for circles/sliders and end time for spinners.

## `.osz` archive boundary

Phase 6.4 uses the pinned Rust `zip` crate with only stored/deflate entries enabled. Inspection occurs before filesystem writes and rejects invalid ZIP structure, encryption, unsupported compression, link-like entries, declared size/count violations, absolute/UNC/drive/device names, parent traversal, Windows reserved/ADS names, duplicate or case-colliding paths, and a file reused as a parent directory. Both declared metadata and bytes observed while streaming are limited; CRC/stream failures and declared-versus-observed size changes are errors.

Extraction only creates a previously absent direct child of an existing parent. Entry paths are reconstructed from validated normal components, files use create-new semantics, and an extraction failure removes the directory created by that operation. After extraction, every beatmap passes the phase 6.2 parser and phase 6.3 semantic validator. A case-insensitive archive-relative index verifies `AudioFilename`, background/video/sample events, and custom hit-sample references without allowing references to traverse out of the archive.

Packaging recursively inventories a trusted source directory without following links, applies the same limits and case-collision rules, and refuses to overwrite an output. Files are written in stable order to a sibling partial file. The finished ZIP is streamed again for CRC and byte-limit verification, atomically renamed, inspected again, and extracted into a disposable verification directory so parser, semantics, and resource references are rechecked. Any failure removes the candidate output and operation-owned staging data.

## State transition

```text
generating -> validating -> packaging -> completed
                       \-> validation_failed
```

The worker's `completed` protocol event means inference completed and transfers ownership of a candidate path to Rust. It must not directly publish the terminal job state after the validator is connected in phase 6.5. A report containing any `error` cannot complete. `warning` and `info` findings may complete as `valid_with_warnings` where the candidate remains otherwise safe.

## Artifact ownership

Each managed job directory uses these reserved paths:

```text
jobs/<job-id>/
|- raw/                    # immutable Mapperatorinator output
|- working/                # disposable extraction and packaging space
|- validated/              # validated or repaired deliverable
|- validation-report.json
`- repair-manifest.json
```

- `raw` is immutable after the inference artifact is moved into it.
- `working` may be deleted after completion, failure, cancellation, or startup recovery.
- `validated` never aliases or overwrites `raw`.
- Report paths are local application data. Diagnostic export must redact personal absolute paths.
- Existing phase 5 path ownership rules continue to apply: no artifact operation may escape the direct managed job directory.

The canonical names and default limits live in `app/src-tauri/src/validation/limits.rs` so the implementation and tests cannot silently diverge from this document.

## Report schema v1

`ValidationReport` is camel-case JSON and contains:

- schema and validator versions;
- job identity, candidate path, and optional validated path;
- terminal validation status;
- structured issues with stable codes, severity, section/line context, user-facing message, and repairability;
- optional beatmap statistics;
- an ordered, auditable repair list;
- UTC start and completion timestamps.

Statuses are `valid`, `valid_with_warnings`, `repaired`, and `invalid`. Severities are `error`, `warning`, and `info`. The report deliberately excludes song bytes, beatmap text, traceback data, and model prompts.

Stable issue codes are represented by the Rust `ValidationCode` enum. New codes may be added without changing schema version; existing meanings must not be repurposed. A breaking field or semantic change requires a new schema version and a database compatibility plan.

## Archive safety limits

Default `.osz` limits are:

| Limit | Value |
| --- | ---: |
| Archive entries | 256 |
| Total declared uncompressed size | 1 GiB |
| Single entry | 512 MiB |
| `.osu` entries | 64 |
| Single `.osu` | 16 MiB |

Before extraction, the archive validator must reject:

- absolute Windows, UNC, device, or POSIX paths;
- `..` traversal after separator normalization;
- names that resolve outside `working` after canonicalization;
- duplicate or case-insensitive-colliding paths;
- symbolic-link or other link-like entries;
- encrypted or unsupported archive features;
- any declared or streamed size beyond the configured limits.

Entry counts and declared sizes are preflight checks, not a substitute for enforcing streamed byte limits while extracting.

## Repair policy

Repairs are deny-by-default. The initial allowlist is limited to byte-order-mark removal, line-ending/final-newline normalization, safe metadata whitespace normalization, unambiguous scalar spelling normalization, and safe archive filename normalization.

The first release must not automatically change hit-object time, position, type, slider geometry, timing/BPM, difficulty values, or playable content. A repair creates a new artifact under `validated`, records before/after/reason in `repair-manifest.json`, and runs the complete validator again. Failure of the repaired copy leaves `raw` untouched and the job non-completable.

## Fixture policy

Fixtures under `app/src-tauri/tests/fixtures/validation` are artificial and small. Hostile archives are described by reviewed JSON recipes and generated only inside a test-owned temporary directory. Tests synthesize any required short WAV; copyrighted music and real user beatmaps are excluded from the repository.
