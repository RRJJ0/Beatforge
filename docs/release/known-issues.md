# Beatforge 0.1.0 known issues and limits

- The generated chart may need manual mapping and timing review even when
  automated validation passes. A failed validation can still expose an import
  action for inspection; it is not a quality approval.
- Generation depends on the pinned Mapperatorinator V32 package and large
  first-run downloads. Offline use requires an already verified local cache.
- CUDA support is hardware-dependent. See `GPU_SUPPORT_MATRIX.md` for tested
  cards and untested configurations; CPU mode is slower.
- A machine without WebView2 may need network access during installer setup.
- An unsigned public build has no verified Windows Publisher. SmartScreen may
  warn and managed Windows policies may refuse it; GitHub hosting and the
  separate updater signature do not remove this limitation.
- The two-version public updater path remains a release-candidate gate until
  recorded in a clean environment.

Report defects using the app's diagnostic export, which omits song contents.
Before sharing, inspect the ZIP for personal information and attach only the
necessary reproduction details.
