# Blender authoring tools

Use `tools/dev.py assets prepare/build` as the normal project entry. Blender runs
in separate background jobs; the static authoring build uses an isolated add-on
configuration. No task resets or saves the user's personal preferences.

- `import_character.py`: verified raw source + explicit static recipe → packed
  authoring candidate; records the scoped compatibility shim and restrictions.
- `inspect_character.py`: read-only image/material/mesh/rig audit.
- `capture_character.py`: declared full-body/face reference captures and provenance.
- `export_character.py`: upstream diffuse/pupil rasterization, independent NPR map
  extraction and runtime-only geometry export → typed profile input.
- `compare_golden.py`: read-only same-toolchain image comparison.
- `inspect_node_math.py`: inspect upstream graph math in a supplied source file.

Contracts live in schemas/authoring. Candidate outputs are separate from accepted
baselines. Reference preparation and comparison commands are documented in
`docs/reference/01-reference-authoring-workflow.md`. The obsolete shell reproducer
and its implicit cache paths have been retired.
