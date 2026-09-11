# Reference Source Register

> This file is the normative replacement for the original Master Plan §8.10.

## V1 reference family

V1 is scoped to a **concrete Genshin reference family**, not a generic “Gacha” shader contract.

Required provenance records include:

- exact Blender version;
- exact Gacha Setup release/tag/commit;
- release package digest;
- concrete Genshin main shader `.blend` artifact digest;
- outline artifact digest;
- post-processing artifact digest;
- Blender render engine and color-management settings;
- exporter settings and contract version;
- profile schema version;
- fixture/camera/light/golden revisions.

## Source roles

| Source | Role |
|---|---|
| Gacha Setup repository/releases | Integration/reference toolchain |
| Bundled Genshin shader artifacts | Concrete upstream visual reference artifacts |
| Blender glTF documentation | DCC export behavior |
| Bevy 0.19.1 glTF source/API | Runtime import behavior |
| glTF 2.0 specification | Custom attribute/extras semantics |
| bevy-stylized ADR/profile docs | Local normative contract |

External sources describe upstream behavior. Local architecture documents define what the framework promises.
