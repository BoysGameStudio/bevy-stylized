# Physical-light color ramps

Status: local Vulkan fixtures pass; application and platform qualification continue. This opt-in backend extends
Bevy StandardMaterial for callers whose color ramps consume physical lighting.
It is separate from the authored-light Character/Hero models.

`physical::PhysicalCelPlugin` registers `PhysicalCelMaterial` and its
`HostMaterial` wrapper after the main Stylized plugin. The default extension
preserves StandardMaterial shading. Setting `params.transform.w` to one and
providing `ramp` enables the physical-light ramp:

1. Evaluate the StandardMaterial light response, including its optical
   properties, texture inputs and shadows.
2. Separate emissive energy from the lit surface.
3. Convert relative luminance to a bounded ramp coordinate and select/blend
   authored bands and rows.
4. Remap the physical-light contribution, add the retained emission and optional
   rim, then apply Bevy's ordinary post-lighting processing.

Host tint multiplies the static StandardMaterial color factor. Host emission
adds to its static emission; an adapter with absolute animated color/emission
can put neutral factors in the immutable base material and publish the complete
animated values in the host record. Ordinary Blend opacity uses the host alpha;
the host still selects the cached alpha pipeline explicitly.

The StandardMaterial uniform stays at binding zero. The extension uses binding
100, with its 80-byte prefix followed by the existing shared host tables when
wrapped (14,928 bytes total). `HostMaterialSource::frame_uniform_binding`
identifies this binding; existing Cel/Character/Hero/Outline implementations
retain binding zero. Base textures and optical properties remain immutable
during host-frame updates.

The backend retains StandardMaterial's resource layout plus the ramp texture and
sampler. Its full stage/texture/sampler budget and platform routing require
measurement; this document makes no Portable-tier claim. Current implementation
targets the forward path. Independent GPU fixtures compare the disabled extension
against StandardMaterial, test host tint/alpha with a non-white metallic base,
and verify that a black ramp suppresses physical light while retaining emission.
The three fixtures pass in `migration-full-gpu-5`. Reflection measures two
material uniforms (192 and 14,928 bytes), seven sampled textures and seven
samplers, with no material storage bindings. The minimal view layout separately
measures four uniforms, three storage buffers, three textures and three samplers.
These measurements do not qualify every optional view feature or device tier.
