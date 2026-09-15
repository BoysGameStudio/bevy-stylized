# Reference and inspector input boundaries

Both tools consume runtime material data, but their input acceptance rules serve
different purposes. R4b preserves the two validators rather than introducing a
shared permissive bundle loader or another product crate.

| Rule | Reference comparison | Inspector |
|---|---|---|
| Entry | Declared character-capture package | Explicit compiled GLB and optional saved recipe |
| Schema | character-capture v1/v2/v3, with version-specific qualification | inspector v1 recipe plus existing runtime sidecar/view schemas |
| Cases | Exact six-case identity/order | Interactive linked A/B state and replay paths |
| Camera | Validated orthographic, 512², recorded actual Blender settings | Perspective orbit/focus, recipe resolution and supported CLI range |
| Sampling | Matched sampling grid and lighting-factor policy for v3 | Normal MSAA 4x; no offline supersampling |
| Paths | Relative inventory paths canonicalized and contained in package root | Reject URL, backslash and non-normal lexical components, then enforce canonical containment |
| Identity | Required reference/calibration/runtime files must be inventoried and match hashes | Hash runtime GLB/sidecar/view, referenced buffers/textures and optional focus; recipe fingerprint must match |
| Geometry count | Exported character mesh primitives used by fixed capture | Recursively count scene 0's primitives for adoption readiness |
| Extra evidence | Calibration, actual camera/light, silhouettes and factor references | Generation/revision, reload retirement, readiness and replay identity |

In particular, canonical containment does not establish the same lexical path
policy. Sharing those implementations without an explicit acceptance-domain
change would silently broaden or narrow one input contract. Shared SHA helpers
alone do not justify a new abstraction. Both already use the product glTF sidecar
parser and neutral capture/readiness harness where those contracts agree.

The reference validator is owned by
[labs/reference character input](../../labs/reference/src/character/input.rs).
The inspector validator remains in
[inspector asset](../../apps/showcase/src/inspector/asset.rs).
R4 moves reference ownership without changing its validation body; the inspector
asset validator is unchanged. Neither transport schema adds an obligation to
support a retired Gacha release.

Reference owns its calibration, case sequence, metrics and qualification report.
Inspector owns options, runtime synchronization, UI, generation-safe capture and
replay. Shader formulas, thresholds and the independently generated Blender
reference remain separate from these transport concerns.
