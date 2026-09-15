# Odette independent source and supported recipe

The HoyoToon catalogue lists character 10000150, Odette / 奥黛塔, model
Avatar_Girl_Sword_Odette, variant Snow Swan's Dance. The checked-in source manifest
records one FBX, eleven material JSON files and forty-one textures with URLs and
SHA-256. Historical comparison established matching model/texture bytes and
semantically equal material JSON; no previous game's checkout is required.

Use the current repository tools:

```sh
python3 tools/dev.py assets prepare --pack genshin-odette
python3 tools/dev.py assets prepare --pack gacha-authoring
python3 tools/dev.py assets build --pack genshin-odette --blender blender
```

The versioned recipe chooses a **static body/face variant**, preserves the original
rest geometry without Rigify/secondary physics, declares excluded effects and
material restrictions, and keeps the full raw source. An authoring-only shim fixes
a verified upstream private-method call without editing the installed add-on.
Blender/Gacha and shader hashes are checked. The isolated build uses the prepared
add-on and a separate process configuration rather than personal preferences.

The build directory contains authoring/character.blend, authoring/audit.json,
reference full-body/face captures with provenance,
runtime/character.glb, runtime/character.genshin.json, compiled/character.glb,
compiled/character.npr.json, compiled/character.view.json and artifact hashes. Pupil composition is rasterized
from the upstream color node output; packed face images come from verified shader
assets. These are project-owned generation steps, not copies of a game's GLB or
processed texture directory.

The static candidate has no missing images, invalid constraints or invalid drivers
in its qualified audit. Material sharing and reload are tested on GPU. This does
not qualify the original full-rig/effect workflow or upstream pixel parity. The
historical full-rig trial and source comparisons live under docs/history; never use
historical commands as current preparation instructions.

The authoring job executes a private temporary add-on copy because the upstream
updater writes state beside its modules. The verified original archive/unpacked
source is not used as a mutable installation. Neutral material light colors and
explicit authored view ambient avoid clipping the reference just to compensate
for a renderer default.
