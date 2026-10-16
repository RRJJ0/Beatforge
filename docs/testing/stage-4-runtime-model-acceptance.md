# Stage 4 Runtime and Model Acceptance

## Automated acceptance

The Rust test suite runs these scenarios without modifying the user's application data or downloading production-sized artifacts:

| Scenario | Expected result | Coverage |
|---|---|---|
| Clean application profile | Manifest starts with runtime and model not installed; a verified model can complete install, verify, and removal | `stage4_acceptance::clean_profile_can_complete_model_lifecycle` |
| Interrupted download | Active operation becomes `failed`, keeps its downloaded byte count, and can transition back to `queued` | `stage4_acceptance::interrupted_download_is_marked_failed_and_can_resume` |
| Interrupted install | Stale staging is removed and the newest backup is restored when the active directory is missing | `install_recovery::tests` |
| Corrupt update | A checksum failure cannot replace the previously installed model | `stage4_acceptance::corrupt_update_never_replaces_previous_model` |
| Corrupt download | Invalid checksum removes the partial artifact and never finalizes it | `downloads::tests::checksum_failure_does_not_finalize_the_artifact` |
| Insufficient disk space | Required size plus safety reserve is enforced before downloading | `downloads::tests::disk_check_keeps_a_safety_reserve` |
| Unsafe removal path | Runtime/model removal refuses paths outside application-managed roots | `runtime::tests` and `models::tests` |
| Manifest update | Installed old version becomes `update_available`; incomplete old downloads are invalidated | `packages::tests` |

Run from `app/src-tauri`:

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
```

Run from `app`:

```powershell
pnpm type-check
pnpm lint
```

## Physical clean-Windows acceptance

This is intentionally manual because it downloads the real private Python runtime, PyTorch wheel, FFmpeg wheel, and approximately 830 MB model checkpoint.

Status: **Passed on 2026-10-01.** The complete installation and restart flow was verified on a physical clean Windows machine.

1. Install the current Windows installer on a Windows 10/11 machine without Python, CUDA Toolkit, FFmpeg, or Git.
2. Open Models and confirm Python Runtime begins as `Not installed`.
3. Install the runtime, close and reopen the application, then verify it remains `Ready`.
4. Run AI environment setup using the recommended backend. Confirm Python, pip, FFmpeg, and PyTorch all become green.
5. Install Mapperatorinator V32. Interrupt the network once, reopen the application, choose Repair, and confirm the download resumes.
6. Verify the installed model and confirm all four files pass checksum verification.
7. Remove the model and confirm its managed directory is removed without affecting any user-selected directory.
8. Reinstall the model, restart the application, and confirm the installed state and path persist.

Record GPU name, driver version, selected PyTorch backend, installer version, and pass/fail notes. Do not mark the physical clean-machine item complete until this run has been performed.
