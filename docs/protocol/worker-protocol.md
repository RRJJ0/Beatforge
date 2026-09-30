# Beatforge Worker Protocol v1

Beatforge desktop core and generation workers communicate over JSON Lines. Each message is one UTF-8 JSON object followed by `\n`. Worker stdout is reserved for protocol messages; diagnostics belong on stderr.

## Transport rules

- Current `protocol_version` is `1`.
- Maximum message size is 64 KiB.
- Unknown versions, malformed JSON, and events for the wrong job terminate the worker.
- The desktop core starts one worker at a time and writes exactly one `start_job` command to stdin.
- Progress events reset the worker inactivity timeout.
- Cancellation is performed by the desktop core terminating the subprocess and waiting for it to exit.

## Start job

```json
{"protocolVersion":1,"type":"start_job","jobId":"BF-123","audioPath":"D:\\Music\\song.mp3","outputPath":"C:\\...\\jobs\\BF-123","title":"Song","artist":"Artist","model":"Mapperatorinator Standard","difficulty":5.2,"scenario":"success"}
```

`scenario` is part of the phase 3 mock worker only. Supported values are `success`, `failure`, `oom`, `hang`, `crash`, and `malformed`.

## Progress

```json
{"type":"progress","protocol_version":1,"job_id":"BF-123","stage":"generating","percent":62,"message":"Generating hit objects"}
```

## Completed

```json
{"type":"completed","protocol_version":1,"job_id":"BF-123","output_path":"C:\\...\\mock-result.json","warnings":["Mock worker output; not a playable beatmap"]}
```

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

