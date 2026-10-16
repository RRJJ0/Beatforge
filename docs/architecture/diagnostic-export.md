# Diagnostic export and privacy boundary

## Purpose

Beatforge diagnostics are designed for troubleshooting installation, hardware, worker, queue, and validation failures without requiring a user to disclose copyrighted media or personal filesystem data. The export is an explicit two-step action: the application first builds and displays a file inventory and exclusion notice; only after the user confirms that preview does it open a save dialog and create a ZIP.

Diagnostic schema v1 contains:

- `manifest.json`: diagnostic ID, UTC creation time, Beatforge version, entry size and SHA-256 inventory, exclusions, and privacy notice.
- `README.txt`: human-readable sharing and privacy guidance.
- `system.json`: OS, architecture, CPU/core count, memory, free disk, GPU, VRAM, and driver metadata.
- `environment.json`: Beatforge version, pinned Mapperatorinator commit, model revision, and allowlisted runtime/model package state without install/download paths or free-text errors.
- `settings.json`: non-path settings only. Output, temporary, and osu! paths are replaced by the literal `excluded` marker.
- `jobs.json`: at most 25 recent pseudonymous job states, progress, difficulty count, validation status/counts/codes, and output availability. It excludes job IDs, title, artist, audio/reference/media paths, descriptors, seeds, error text, provenance paths, and artifacts.
- `recent-logs.json`: metadata for at most eight bounded regular `.log` files and counts of known stable error signals. Raw log bytes, lines, filenames, tracebacks, and arbitrary error text are never included.

## Security properties

The backend constructs every entry from typed allowlists; it never starts with a directory tree and subtracts files. Log candidates must be direct regular non-symlink `.log` files no larger than 2 MiB, and only structural metadata plus counts for explicitly known codes are retained. Package IDs are restricted to Beatforge's built-in runtime and model IDs. Settings and job status strings are mapped to known values, so a modified SQLite row cannot inject arbitrary text into the ZIP.

The export must use `.zip`, requires the preview-issued `BF-DIAG-*` identity, refuses to overwrite an existing file, writes through a same-directory partial file, and removes an incomplete partial on failure. It never changes jobs, settings, logs, models, or generated artifacts.

Automated tests seed settings, SQLite fields, validation messages, and logs with a personal username, song/artist names, media paths, descriptors, and job ID. They reopen the resulting ZIP and assert that none of those values occur, while stable validation and CUDA error codes remain useful.

## Public issue guidance

The GitHub bug and hardware templates request a Beatforge revision, optional diagnostic ID, reproduction steps, expected/actual behavior, and hardware measurements. They explicitly reject songs, beatmaps, media, databases, raw logs, credentials, and personal paths. A diagnostic ZIP should still be reviewed by its owner before sharing; schema v1 reduces disclosure risk but is not a substitute for user judgment.

Repository-history credential and private-data scanning is a release-owner operation and remains required before changing the repository from Private to Public Preview. It is not performed or represented by the in-app diagnostic exporter.
