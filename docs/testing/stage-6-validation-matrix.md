# Stage 6 output validation matrix

## Phase 6.1 contract gates

| Area | Case | Expected result |
| --- | --- | --- |
| Wire format | Serialize schema v1 report | camelCase fields, snake_case status/severity, stable SCREAMING_SNAKE_CASE code |
| Completion | Report contains an error | `can_complete()` is false regardless of status |
| Completion | Valid map contains warnings | `valid_with_warnings` may complete |
| Limits | Default limits | Match the documented entry and byte ceilings |
| Artifact names | Reserved paths | Relative single components only |
| Fixtures | Repository contents | No user music, generated model output, or opaque hostile ZIP |

## Parser and semantic matrix for phases 6.2-6.3

| Fixture or case | Expected code/status |
| --- | --- |
| `valid/minimal-standard.osu` with synthesized audio | `valid` |
| `warnings/incomplete-metadata.osu` | `METADATA_INCOMPLETE`, warning |
| `invalid/missing-hitobjects.osu` | `SECTION_MISSING`, error |
| `invalid/invalid-timing.osu` | `TIMING_INVALID`, error |
| `invalid/invalid-slider.osu` | `HIT_OBJECT_INVALID`, error |
| `invalid/out-of-bounds.osu` | `OBJECT_OUT_OF_BOUNDS`, policy-selected warning/error |
| Invalid UTF-8 input | `OSU_INVALID_ENCODING`, error |
| Unsupported format or gamemode | `OSU_FORMAT_UNSUPPORTED`, error |
| Referenced audio absent from package | `AUDIO_MISSING`, error |

Phase 6.2 additionally verifies UTF-8 BOM and CRLF acceptance, invalid UTF-8 rejection, pre-read file size enforcement, unsupported/missing headers, duplicate known section/key rejection, unknown section/field preservation, partial result collection, stable line context, and deterministic results for malformed input. `invalid-timing.osu`, `invalid-slider.osu`, and `out-of-bounds.osu` intentionally pass the 6.2 structural layer; their semantic errors are asserted when phase 6.3 is connected.

Phase 6.3 verifies the valid fixture's derived 120 BPM and three typed objects; inherited timing/SV; timing and hit-object ordering; zero/incorrect beat length; the required uninherited timing point; difficulty ranges; supported object and hit-sound flags; object-origin bounds; slider curve/control-point/repeat/length rules; spinner end time; and optional audio-duration warnings. Out-of-playfield slider control points produce warnings without discarding an otherwise parseable slider.

## Archive matrix for phase 6.4

`hostile/archive-cases.json` defines the reviewed cases mirrored by archives generated in a temporary directory at test time.

| Recipe | Expected code |
| --- | --- |
| `path-traversal` | `ARCHIVE_PATH_TRAVERSAL` |
| `absolute-windows-path` / `absolute-posix-path` | `ARCHIVE_ABSOLUTE_PATH` |
| `case-collision` | `ARCHIVE_CASE_COLLISION` |
| `entry-count-limit` | `ARCHIVE_LIMIT_EXCEEDED` |
| `link-entry` | `ARCHIVE_LINK_ENTRY` |
| Malformed ZIP directory | `ARCHIVE_INVALID` |
| Valid archive with no `.osu` | `BEATMAP_NOT_FOUND` |
| Stream exceeds declared/configured ceiling | `ARCHIVE_LIMIT_EXCEEDED`; partial working data removed |

Phase 6.4 automated coverage additionally verifies malformed/non-ZIP input, archives without a beatmap, file-as-parent collisions, missing audio and background references, extraction into create-new paths, stable repackaging, streamed CRC/size verification, full post-package extraction/parse/semantic/resource validation, no overwrite, and cleanup of rejected package, partial, and verification artifacts.

## Lifecycle and repair matrix for phases 6.5-6.8

| Scenario | Required behavior |
| --- | --- |
| Worker returns candidate | Preserve under `raw`; enter `validating`, not `completed` |
| Validator reports error | Persist report; enter `validation_failed`; do not publish import action |
| Validator reports warning only | Persist report; permit completed result with visible warnings |
| Allowlisted repair succeeds | Write new `validated` artifact and manifest; revalidate; preserve `raw` |
| Repair fails | Preserve `raw`; remove incomplete validated copy; remain non-completable |
| App restarts | Report, artifact identity, and job state remain consistent |
| Open/import action | Only a managed, validated path may reach the operating system |

Phase 6.5 automated coverage verifies that a valid candidate is moved from the worker location to `raw`, copied to `validated`, and accompanied by a round-trippable JSON report; an invalid candidate keeps `raw` but publishes no validated artifact; and a candidate outside the managed job root is reported without being moved. Database migration coverage verifies schema v3 is idempotent and that report JSON, terminal status, warning state, and validated output path are persisted together. The mock worker now emits a structurally valid `.osu`, so the desktop development path passes through the same pipeline instead of bypassing validation.

Phase 6.6 coverage verifies BOM, CRLF/bare-CR, final-newline, known Metadata whitespace, and equivalent scalar normalization while asserting timing and hit-object lines remain byte-identical. Pipeline tests compare the preserved raw bytes, repaired deliverable, report repairs, applied/failed manifest, working-directory cleanup, and non-publication after failed revalidation. Archive coverage uses backslash-separated safe entries plus a repaired `.osu`, then requires the repackaged `.osz` to pass a fresh archive inspection and complete extraction validation.

## Commands

Phases 6.1-6.5 are checked with:

```powershell
cd app
pnpm type-check
cd src-tauri
cargo test validation::
cargo clippy --all-targets -- -D warnings
```

Later phase 6 steps add parser, archive, persistence, and physical osu! stable/lazer acceptance commands to this matrix.
