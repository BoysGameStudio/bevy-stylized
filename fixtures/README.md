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
