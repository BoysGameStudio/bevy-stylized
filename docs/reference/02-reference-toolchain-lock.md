# Current reference toolchain identity

The one qualified authoring source is locked in
[assets/sources/gacha-authoring.json](../../assets/sources/gacha-authoring.json).
The default recipe is [genshin-static-emission-v1.json](../../assets/recipes/genshin-static-emission-v1.json),
including source outlines, metal/ramp, optimized normals and emission. Reduced-feature
recipes use the same source lock and remain explicit regression controls.
There is no older synthetic-reference toolchain or version-compatibility matrix.

Record the exact Blender build, addon release/archive hash, source/model scope,
material maps, recipe, evaluated camera/light state, display transform, resolution,
sampling grid, compiler/lock/source identity and actual renderer adapter for a
qualified case. A source upgrade requires a newly verified current input; it does
not create a requirement to keep old versions runnable.

The [current authoring workflow](01-reference-authoring-workflow.md) and declared
character-case manifests own reproduction. A manifest must identify real inputs;
missing inputs or a failed pixel metric cannot be relabelled as success.
