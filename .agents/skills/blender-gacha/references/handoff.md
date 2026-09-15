# Handoff to runtime work

A handoff is an evidence bundle, not a new runtime JSON schema. Put it in a
local candidate directory and name the files explicitly in the delivery.

Minimum information:

- Original source manifest: game, identity, variant/channel, URLs and hashes.
- Actual Blender, Gacha Setup and shader artifact versions/hashes; import options.
- Candidate `.blend`, texture dependencies or packed image status, linked libraries.
- Audit JSON and import log, with material/rig findings classified.
- Reference PNGs plus camera, pose, lighting/palette, color management and resolution.
- Object/material selection and intended runtime behavior, including effects,
  transparency, animation/physics, outlines, measured units and forward.
- Known limitations and what was actually tested. New import, old game asset and
  synthetic comparison fixture must have distinct identities.

The receiver (`bevy-stylized`) chooses the export adapter and validates its
contract. Gacha node graphs, Rigify controls, drivers, geometry-node outlines,
compositing and physics are not automatically represented by GLB.

Keep Blender's reference scene intact. Prepare runtime geometry/materials in a
separate candidate or transient process. Do not bake the upstream hull outline
into exported geometry and then add a second runtime outline. Do not claim that
`export-sidecar.json` is already the runtime `<asset>.npr.json`.

No global total ordering is required: an asset-only inquiry ends with provenance;
a shader debug task may start from an existing blend. Read only the relevant
references and continue the user-authorized scope across the handoff.
