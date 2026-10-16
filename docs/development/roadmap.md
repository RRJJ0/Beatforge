# Development milestones (overview)

[繁體中文詳細紀錄](roadmap.zh-TW.md) · [Developer index](README.md)

This overview accompanies the preserved, detailed Traditional Chinese
planning/acceptance log. It is a historical record, not a list of currently
available features. For the product as shipped, see the [user README](../../README.md).

| Stage | Focus | Outcome / next boundary |
| --- | --- | --- |
| 0–1 | Product definition, draw.io wireframes, Vue prototype | Established the desktop information architecture and interaction language. |
| 2–3 | Tauri shell and worker protocol | Introduced local capabilities and a bounded JSON-lines worker contract. |
| 4 | Runtime and model management | Added pinned downloads, checksums, repair, recovery, and compatibility checks. |
| 5 | Mapperatorinator V32 integration | Added the production worker, request adaptation, progress, cancellation, and provenance. |
| 6 | Output validation and osu! import | Added safe archive handling, `.osu` validation, audited repair, and non-destructive import handoff. |
| 7 | History, settings, localization, and product polish | Added storage management, notifications, lightweight preview, English/Traditional Chinese UI, and daily-flow acceptance. |
| 8 | Beatmap sets and media | Added multi-difficulty output, background image/video support, and readable `.osz` names. |
| 9 | Testing and distribution | Added release gates, diagnostic export, legal notices, Windows installer, updater, GPU matrix, and release-candidate process. Final public-endpoint and clean-machine acceptance remain separate human checks. |

Future ideas recorded in the detailed log include additional beatmap controls,
reviewed multiple-model support, LoRA/custom checkpoints, other game modes,
and non-NVIDIA platforms. They are not current product promises.
