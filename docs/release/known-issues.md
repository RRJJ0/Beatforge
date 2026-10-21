# Beatforge 0.1.0 known issues and limits

[繁體中文](known-issues.zh-TW.md) · [Home](../../README.md)

- The generated chart may need manual mapping and timing review even when
  automated validation passes. A failed validation can still expose an import
  action for inspection; it is not a quality approval.
- Generation depends on the pinned Mapperatorinator V32 package and large
  first-run downloads. Offline use requires an already verified local cache.
- CUDA support is hardware-dependent. See the [GPU support matrix](../testing/stage-9-gpu-support-matrix.md) for tested
  cards and untested configurations; CPU mode is slower.
- A machine without WebView2 may need network access during installer setup.
- An unsigned public build has no verified Windows Publisher. SmartScreen may
  warn and managed Windows policies may refuse it; GitHub hosting and the
  separate updater signature do not remove this limitation.
- The production GitHub updater endpoint and signed `latest.json` are public,
  but evidence for a clean-environment `0.0.9` to `0.1.0` update through that
  production endpoint has not been recorded. The isolated HTTPS updater path
  passed user-reported acceptance before release.

Report defects using the app's diagnostic export, which omits song contents.
Before sharing, inspect the ZIP for personal information and attach only the
necessary reproduction details.
