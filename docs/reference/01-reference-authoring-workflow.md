# Independent reference and authoring workflow

> Status: Current normative workflow. Historical migration-renderer instructions
> are retained under docs/history and are not active prerequisites.

There are two renderer sources and three forms of evidence: upstream Blender/Gacha
behavior, regression against accepted production images, and independent semantic
fixtures/assertions. The current production image is never used to manufacture its
own upstream truth. The retired engine-reference implementation is not compiled.

## Current upstream acceptance

Only the latest stable Gacha Setup release is a support target. Verified on
2026-09-15: [v3.6.4](https://github.com/PaoloESAN/gacha-setup/releases/tag/v3.6.4).
The exact qualified archive/hash is pinned in `assets/sources/gacha-authoring.json`;
ordinary builds do not silently download a moving version. When upstream releases
a new stable version, qualify a fresh candidate and advance this single target.
No compatibility matrix for retired Gacha versions is required.

```sh
python3 tools/dev.py assets prepare --pack genshin-odette
python3 tools/dev.py assets prepare --pack gacha-authoring
python3 tools/dev.py assets build --pack genshin-odette --blender blender
python3 tools/dev.py check --lane character-reference --required --case BUILD/character-matched/case.json --out target/stylized/NEW_RUN
```

The current static scope and unchanged image gates are defined by the
[capture contract](09-static-character-capture.md). The isolation gate rebuilds
this workflow from raw inputs and requires strict parity to pass. A rendering
failure is not accepted merely because it was already known.

## Historical reference evidence

`gacha-reference`, `assets build --pack reference` and `check --lane reference`
retain explicit access to the locked v3.6.2 experiment for historical investigation.
They are not part of default acceptance, do not require repair and are not current
project blockers. Dated failure reports remain unchanged. The historical
`reference-toolchain.toml` must not be used as the current authoring version lock.

## Authoring and runtime artifacts

Source manifests identify original assets and tool archives. Recipes state mesh,
material, pose and feature scope. Authoring outputs preserve the packed reference
blend and audit; runtime export uses a separate transient scene without upstream
outline geometry and widgets. Strict profile compilation creates typed runtime
material data and an atomic new bundle. See the [integration guide](../authoring/05-integration-guide.md).

## First-party regression

The mandatory public lane uses self-made geometry and the self-made skinned/morph
character. Accepted images live in fixtures/golden; diagnostic output is separate.
External art and historical snapshots remain under ignored assets/local. See
[comparison criteria](08-golden-tolerance-and-cross-layer-criteria.md) and
[upgrade policy](04-upstream-contract-upgrade-and-golden.md).
