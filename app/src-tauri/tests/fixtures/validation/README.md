# Validation fixtures

These fixtures are intentionally tiny, artificial, and free of user music or generated model output.

- `valid/` contains the minimum osu!standard structure accepted by the phase 6 contract.
- `warnings/` contains parseable maps that should complete with explicit warnings.
- `invalid/` contains deterministic structural or semantic failures.
- `hostile/archive-cases.json` contains recipes for generating unsafe ZIP entries in a test-owned temporary directory.

Hostile `.osz` files are generated during the archive tests instead of being stored as opaque binaries. This keeps their dangerous path names visible during review and ensures extraction tests cannot accidentally target a repository-relative path.

`fixture.wav` will be synthesized by the test harness when audio-reference validation is implemented. The repository does not contain copyrighted audio.
