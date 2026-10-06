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

## Archive matrix for phase 6.4

`hostile/archive-cases.json` is used to generate the archive in a temporary directory at test time.

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

## Commands

Phases 6.1-6.2 are checked with:

```powershell
cd app
pnpm type-check
cd src-tauri
cargo test validation::
cargo clippy --all-targets -- -D warnings
```

Later phase 6 steps add parser, archive, persistence, and physical osu! stable/lazer acceptance commands to this matrix.
