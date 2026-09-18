# Opt-in host presentation inputs

Status: implemented diagnostic interfaces; local CPU/GPU scopes below are
qualified separately from application migration or cross-platform rollout.

`HostMaterialPlugin` registers `HostMaterial<M>` for Cel, Character, Hero and
Outline. Construct a shared static material with `HostMaterial::new(base,
HostAlphaMode::Opaque)` or an explicit `Blend` variant. Attach `HostInstanceInput`
to each rendered mesh and `HostViewInput` to its referenced view entity. Hosts
own those inputs and must call `publish_host_inputs` after their final pose.
`HostMaterialScope` seeds inputs during descriptor adoption; subsequent scope
changes do not automatically replace the per-mesh presentation inputs.

The transport packs 128 distinct 96-byte instance records and 16 distinct
160-byte view records after the existing material prefix. Identical values share
slots. It uses no extra uniform binding and no per-frame material-asset mutation.
Current prefixes plus tables fit below 16 KiB (Hero: 15,488 bytes; Outline: 14,976 bytes). Slot assignment
and frame data publish together. Non-finite values, invalid indices and capacity
overflow reject the whole frame; `HostFrameStatus` reports the failure while the
last valid frame remains installed. Hosts must prevent capture/readiness from
accepting a rejected generation. This resource is not a GPU completion fence.

## Scheduling and fullscreen ownership

`configure_presentation_schedule(app, label)` selects one externally run schedule
for outline/deformation synchronization, face and instance packing. It disables native presentation synchronization and does not advance a
clock, animation or physics solver. Call that schedule only after the host's final
pose and final source GlobalTransforms. Renderer-owned hull globals are copied
from their source; the host hierarchy is not propagated again. The host owns
camera, lighting and exposure in this mode.

If the host owns its fullscreen pass, insert `ExternalPostProcess` **before**
adding `StylizedPlugin`. This prevents Stylized from registering a competing
`FullscreenMaterialPlugin`. The pinned Bevy helper uses a non-generic pipeline-id
component: even a second type with no active effect can remove another type's
pipeline. Choosing an external presentation schedule alone does not opt out of
fullscreen registration.

Each input currently selects a view explicitly. The host must prove that its mesh
is visible only in that view (for example, separate scene instances and render
layers). General same-entity/multiple-camera routing is not implemented. Alpha
pipeline variants must be selected explicitly; tint opacity does not change the
material pipeline. The interfaces do not yet provide a complete alpha-variant
cache, generation lifecycle or missing-input visibility policy.

## Complete skinning

The glTF adapter registers the stable `CEL_JOINTS_1` / `CEL_WEIGHTS_1` aliases.
Surface, hull and prepass vertex paths add the secondary four influences after
morphing. `skin_vertex_positions` and complete mesh bounds include both sets;
malformed/incomplete pairs produce diagnostics. CPU inspection callers remain
responsible for supplying the evaluated morph positions when applicable.

## Local evidence and limits

- Render library: 53 unit tests pass; Clippy with `-D warnings` passes.
- `migration-full-gpu-4`: 28 required cases across all 12 registered GPU binaries and the synthetic-import
  tail pass on the local Vulkan device.
- Complete-skin fixtures compare GPU surfaces/hulls against independently baked
  geometry with and without morphs for Cel, Character and Hero. Prepass pipelines
  run; numerical motion-vector texture agreement remains unqualified.
- Host-material GPU fixtures test shared Cel/Character/Hero material immutability,
  independent tint updates, alpha-zero blending and opaque restoration.
- Full application roles, source outline semantics, transparent-overlap sorting,
  replacement/retry/lifecycle, performance and other platforms remain separate
  acceptance work. These checks do not accept new golden images.

## Explicit opaque composition

`HostMaterial::with_opaque_priority` supplies a stable CPU-side layer priority,
independent of bind-group allocation. It does not add a shader binding or change
the host-record ABI. `OutlineOrder` can retain engine order or place library hulls
before/after other surfaces. Priorities are applied after Bevy's built-in phase
sort and before batching constructs its matching buffers. Unrelated non-host
materials do not inherit host priorities merely because their shader matches.

The opt-in source outline mode uses local-space extrusion, a declared color
exponent and five alpha-band-selected colors. Generic pixel-width outlines remain
available. Source prepared-ramp and normal conventions are explicit material
parameters; they are not inferred from a character name by the library.

An independent GPU fixture verifies that swapping material allocation order does
not change the winner of two coplanar surfaces with explicit priorities, and
that changing the priority does change the winner. Application composite-image
qualification remains separate from this mechanism test.
