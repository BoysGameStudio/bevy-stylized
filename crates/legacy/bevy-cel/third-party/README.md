# Bevy prepass shader

`../src/shaders/character_prepass.wgsl` adapts the vertex portion of Bevy
0.19.1's `bevy_pbr/src/prepass/prepass.wgsl`. The Bevy source is dual licensed
under MIT OR Apache-2.0; this distribution includes the
[MIT license](Bevy-LICENSE-MIT).

The local changes read CEL's second skin influence set for both current and
previous-frame deformation. Update this shader against the locked Bevy prepass
implementation when upgrading Bevy.
