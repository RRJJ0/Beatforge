# Beatforge Worker Protocol v1

Beatforge desktop core and generation workers communicate over JSON Lines. Each message is one UTF-8 JSON object followed by `\n`. Worker stdout is reserved for protocol messages; diagnostics belong on stderr.

## Transport rules

- Current `protocol_version` is `1`.
- Maximum message size is 64 KiB.
- Unknown versions, malformed JSON, and events for the wrong job terminate the worker.
- The desktop core starts one worker at a time and writes exactly one `start_job` command to stdin.
- Progress events reset the worker inactivity timeout.
- Cancellation is requested with a versioned `cancel_job` frame. Rust waits briefly for a graceful exit, then terminates and reaps the process tree.

## Start job

```json
{"protocolVersion":1,"type":"start_job","jobId":"BF-123","audioPath":"D:\\Music\\song.mp3","outputPath":"C:\\...\\jobs\\BF-123","title":"Song","artist":"Artist","model":"Mapperatorinator Standard","difficulty":5.2,"scenario":"success"}
```

`scenario` is part of the phase 3 mock worker only. Supported values are `success`, `failure`, `oom`, `hang`, `crash`, and `malformed`.

Production commands use snake-case protocol fields and contain only adapter-approved absolute paths, the pinned source/model identities, detected GPU metadata, and a whitelisted `config` object. Vue cannot supply a checkpoint or output path.

## Progress

```json
{"type":"progress","protocol_version":1,"job_id":"BF-123","stage":"generating","percent":62,"message":"Generating hit objects"}
```

## Completed

```json
{"type":"completed","protocol_version":1,"job_id":"BF-123","output_path":"C:\\...\\result.osz","warnings":[],"provenance":{"schema_version":1,"job_id":"BF-123","reproduction_key":"sha256...","source":{"mapperatorinator_commit":"0e2b...","worker_version":"1","protocol_version":1},"model":{"package":"mapperatorinator-v32-standard","version":"32..."},"runtime":{"pytorch_version":"2.10.0+cu128","cuda_runtime":"12.8","device":"cuda","precision":"fp32"},"parameters":{"seed":12345},"input":{"audio":{"sha256":"..."}},"output":{"sha256":"..."},"timing":{"elapsed_seconds":123.4},"known_limitations":["osu!standard only"]}}
```

`provenance` is required for the production worker and omitted by the mock worker. Rust stores it in the job's existing `result_json` column before changing the job to `completed`. A missing or unpersistable production record is a terminal protocol failure.

## Failed

```json
{"type":"failed","protocol_version":1,"job_id":"BF-123","code":"CUDA_OUT_OF_MEMORY","message":"GPU memory is insufficient for this job","recoverable":true}
```

## Process lifecycle

1. Vue calls the typed `create_job` Tauri command.
2. Rust validates and stores the request in SQLite with `queued` status.
3. The process manager starts the next subprocess and records its PID.
4. stdout events update SQLite and are forwarded to Vue as `job-event` events.
5. stderr is appended to `logs/job-<id>.log`.
6. Completion, failure, cancellation, timeout, protocol errors, and unexpected exits become terminal job states.
7. Jobs left active during application shutdown are marked `interrupted` and can be retried.

