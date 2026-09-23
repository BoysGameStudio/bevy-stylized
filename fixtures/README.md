# Test fixtures

- `synthetic/reference-scene/`: first-party fixed camera/body/head/hair/ground scene,
  its Blender generator outputs and export provenance.
- `synthetic/character/`: independently generated two-joint, three-material GLB
  with a face morph and animation; `tools/assets/synthetic.py` rebuilds it.
- `golden/`: existing first-party accepted baselines relocated byte-for-byte after
  pre-migration GPU comparison passed. Third-party character captures are excluded.

Third-party raw/authoring/runtime packs and historical character images live under
ignored `assets/local`. Diagnostic captures and reports use `target/stylized`.
Tests never create a missing accepted baseline as an implicit successful comparison.

## Golden platform overlays

The canonical golden host is Linux/Vulkan (docs/reference/08 §1); baselines at
`golden/<set>/<case>.png` are accepted there. A platform overlay at
`golden/<set>/<platform>/<case>.png` (platform = `<os>-<backend>`, e.g.
`macos-metal`) holds **only** the cases whose deltas against the canonical
baseline are root-caused platform truth (device/driver evidence linked from the
overlay's acceptance record), each with the standard `.golden.json` provenance
sidecar carrying the capturing device, GPU/driver, engine commit and timestamp.

Resolution (`tests/harness/src/golden.rs`): comparison reads the overlay case
when present, otherwise the canonical baseline. A missing overlay case is never
a silent pass — the canonical baseline decides, and on a platform where the
capture genuinely differs the comparison fails until the overlay is explicitly
captured, reviewed and accepted. The platform identity comes from the actual
render adapter (OS + backend), never from a flag, so a capture cannot
masquerade under another platform's overlay. On the canonical host the overlay
is never consulted.

Capture/acceptance stay separate acts: `GOLDEN_REBASELINE=1` on a
non-canonical platform writes a failing case to the overlay as a candidate
(canonical baselines are never overwritten off the canonical host); the
candidate becomes a baseline only through review and commit, recorded in the
overlay's acceptance note (`golden/<platform>.acceptance.md`).
