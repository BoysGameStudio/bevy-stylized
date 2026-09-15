# Upstream qualification, upgrades and Golden provenance

> Status: Current normative workflow; unresolved family visual criteria remain
> tracked by ADR-0007. This policy does not accept a new baseline automatically.

Gacha Setup support targets only the latest stable release. Each qualified target
is pinned by version and hash; old versions become historical evidence rather
than ongoing compatibility obligations.

An upgrade has separate installation, authoring, export, runtime and image gates.
Record exact Blender/Gacha/shader versions, source file hashes, settings, recipe
revision, graph signature, model scope and output hashes. A plugin installed on a
machine does not change node groups already stored in a Blender file.

Create a distinct candidate. Compare node interfaces/topology, source-map bindings,
rig and material audit, runtime material coverage and relevant image views. Do not
weaken signatures or use fixture mode to force a real character through export.
Required missing textures, version mismatches, unmapped roles and active unsupported
features must be reported explicitly.

## Evidence identity

Golden provenance includes case and source identities, renderer/toolchain, schema,
repository/source revision, GPU/backend/driver, resolution and color/camera/pose
settings. File SHA-256 identifies artifacts; pixel hashes or image comparison
identify rendered equality independently of PNG metadata. Existing FNV identifiers
in historical first-party harness sidecars remain readable; they are not relabeled
as cryptographic SHA-256.

An accepted production baseline answers a regression question, not whether a family
matches upstream. The same current production output cannot become both sides of
an upstream comparison. A file missing from a baseline set is BLOCKED, never an
implicit baseline acceptance. Capture, compare and accept are distinct operations.

## Change classification

Record whether a delta comes from upstream behavior, the export/data boundary,
renderer implementation, hardware/driver/color-management differences, an explicit
quality fallback, or unsupported semantics. Preserve the original result and case
scope; removing an effect or changing input maps creates a different declared case.
Do not change denominators or classify a requested missing case as passed.

The former three-renderer migration evidence is archived. Its source attribution
and known visual failures remain available, but no active workflow requires that
renderer, another game checkout or its processed assets.
