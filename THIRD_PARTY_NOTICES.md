# Beatforge third-party notices

This file applies to Beatforge 0.1.0. Beatforge itself is available under the
MIT License in `LICENSE`. That license does not relicense third-party software,
model weights, media, or user content.

The versioned, machine-readable inventory is
`third_party/dependency-inventory.json`. It records the exact package version,
declared license, author or project URL when available, and whether a component
is shipped with Beatforge or downloaded into the private runtime after the user
starts setup. The Rust and frontend inventories are regenerated from
`Cargo.lock` and `pnpm-lock.yaml`; the Python inventory is checked against the
hash-pinned requirements file.

## Components downloaded after installation

These components are not embedded in the Beatforge installer:

- **Mapperatorinator**, pinned to commit
  `0e2b0e387aab4b35c64b0b11b12d47578dea7587`, is Copyright (c) 2024
  OliBomby and is licensed under MIT. Source and license:
  <https://github.com/OliBomby/Mapperatorinator/tree/0e2b0e387aab4b35c64b0b11b12d47578dea7587>.
- **slider**, pinned to commit
  `ea28c484a0a542d6e094e25aa02e1529ff2c8313`, is licensed under
  LGPL-3.0-or-later. Source and license:
  <https://github.com/OliBomby/slider/tree/ea28c484a0a542d6e094e25aa02e1529ff2c8313>.
  Beatforge does not modify or statically link slider; pip installs it into the
  user-owned Python environment. Users may replace that installation with a
  compatible build.
- **Mapperatorinator-v32 model weights**, pinned to revision
  `74f22583400d259bf424819e11027c17933efe54`, declare MIT in the repository
  metadata: <https://huggingface.co/OliBomby/Mapperatorinator-v32/tree/74f22583400d259bf424819e11027c17933efe54>.
  The model card names `project-riz/osu-beatmaps` and
  `project-riz/osu-beatmap-tags` as datasets but does not provide enough detail
  for Beatforge to independently approve redistribution. Beatforge therefore
  downloads the pinned model after setup begins and does not mirror or bundle
  it in release artifacts.
- **CPython 3.10**, **PyTorch 2.10.0**, **torchaudio 2.10.0** and
  **imageio-ffmpeg 0.6.0** are installed into the managed runtime. Their source
  and license information is linked from the dependency inventory. The
  imageio-ffmpeg wheel contains a platform FFmpeg executable; the exact FFmpeg
  build and its effective LGPL/GPL configuration must be recorded from
  `ffmpeg -version` for each release candidate.

Downloaded components keep their own license terms. The application stores
them in its managed data directory and verifies pinned versions or hashes; it
does not claim ownership of them.

## Components shipped in the desktop application

The Tauri/Rust binary and compiled web frontend contain packages listed under
`rust` and `npm` in the inventory. Their declared licenses include permissive
licenses such as MIT, Apache-2.0, BSD, ISC, Unicode and zlib-style licenses.
Package names, versions, declared license expressions, authors and upstream
links in the inventory are part of this notice. Upstream copyright notices and
license files remain authoritative.

Release packaging must include this file, Beatforge's `LICENSE`, and the exact
`dependency-inventory.json` generated from the release commit. A missing or
unknown license, prohibited redistribution term, changed lockfile, unreviewed
FFmpeg build, or model/runtime file appearing in the installer is a release
blocker. Automated output supports but does not replace human review.

## AI generation, osu! and user content

Beatforge uses Mapperatorinator to generate beatmap content. Generated work
should be clearly disclosed as AI-assisted and shared as Work in Progress while
it is being reviewed. Current osu! ranking criteria require hit objects,
hitsounds and timing to be based on direct human input, and also restrict
AI-generated media for ranked beatmaps. Rules can change; users must check the
current osu! rules before uploading.

Users are responsible for permission to use and upload songs, artwork, video,
hitsounds and other input media. Beatforge is an independent project and is not
affiliated with, endorsed by, or sponsored by osu!, ppy Pty Ltd, OliBomby, or
the third-party projects listed above.

This notice is informational and is not legal advice.
