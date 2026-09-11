# Third-Party Notices and Reference Provenance

This repository contains architecture text authored for the Bevy Stylized project plus legacy runtime crates imported from the Tenacity project. It does **not** vendor proprietary game models/textures or the Gacha Setup/Blender shader source files.

Imported code:

- `crates/legacy/bevy-cel` and `crates/legacy/bevy-cel-metadata` — copied from the Tenacity project (same author lineage), originally MIT licensed; see `crates/legacy/README.md` for provenance;
- `crates/legacy/bevy-cel/src/shaders/character_prepass.wgsl` — adapts Bevy 0.19.1's `bevy_pbr` prepass shader (MIT OR Apache-2.0); the MIT license text is bundled in `crates/legacy/bevy-cel/third-party/`.

Reference/provenance sources discussed by the documentation include:

- Gacha Setup for Blender — upstream project declares GPL-3.0;
- Festivity / Blender-miHoYo-Shaders — referenced Genshin Blender shader lineage, GPL-3.0 according to its repository;
- Blender documentation and glTF exporter behavior;
- Bevy 0.19.1 source/API documentation;
- wgpu/WebGPU/glTF specifications and documentation.

These entries document provenance and technical reference relationships. They are **not** a final license determination for a future runtime repository.

Before publishing runtime/source artifacts, close ADR-0017 (`License and Provenance`) and perform a dedicated license/provenance review of any copied, modified, generated, bundled or redistributed material.

No unlicensed commercial-game asset should be added to the public fixtures or CI requirements.
