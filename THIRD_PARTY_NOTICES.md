# Third-Party Notices and Reference Provenance

The active source is the independent Bevy Stylized framework and its tooling.
It does not vendor proprietary game models/textures or Gacha/Blender shader assets.
Downloaded reference inputs and derived art remain in ignored assets/local.

Historical code provenance:

- The former bevy_cel / bevy_gltf_metadata reference crates were imported from
  Tenacity at b5b26ac with their original MIT declarations, then retired from the
  active tree by the independent-project migration. Their source and notices remain
  in Git history; see [current attribution](THIRD_PARTY_NOTICES.md).
- The former character prepass adaptation referenced Bevy 0.19.1 (MIT OR Apache-2.0).
  Retirement does not remove its historical attribution or establish the license
  of any other source in the current repository.

Reference/provenance sources discussed by the documentation include:

- Gacha Setup for Blender — upstream project declares GPL-3.0;
- Festivity / Blender-miHoYo-Shaders — referenced Genshin Blender shader lineage, GPL-3.0 according to its repository;
- Blender documentation and glTF exporter behavior;
- Bevy 0.19.1 source/API documentation;
- wgpu/WebGPU/glTF specifications and documentation.

These entries document provenance and technical reference relationships. They are **not** a final license determination for a future runtime repository.

Before publishing runtime/source artifacts, close ADR-0017 (`License and Provenance`) and perform a dedicated license/provenance review of any copied, modified, generated, bundled or redistributed material.

No unlicensed commercial-game asset should be added to the public fixtures or CI requirements.

Development inspector provenance (2026-09-15): camera/viewport interaction and
capture cancellation behavior were informed by Tenacity's `view-ink` Showcase at
commit `85565fd65a5feed57b0ef4898b049eb4e2c711a2`, whose workspace declares
MIT OR Apache-2.0. The selected worktree hashes and migration assessment are recorded
in `docs/quality/tenacity-reuse-inventory-2026-09-15.json` as the current source-attribution record. The new inspector uses this
repository's material facade and capture harness; no Tenacity crate or art is
vendored. Its optional UI uses `bevy_egui` 0.41.1 and dependency-bundled fonts.
