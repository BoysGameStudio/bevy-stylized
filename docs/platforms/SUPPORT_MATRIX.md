# Platform Support Matrix

> **Status:** Evidence ledger. `Supported` is never inferred from compile success.

## Evidence states

- ✅ **Validated** — evidence exists for this cell.
- 🧪 **Experimental** — partial implementation/evidence exists.
- ⚪ **Not tested** — no qualification evidence yet.
- 🚧 **Blocked** — known blocker or unresolved ADR prevents qualification.
- ❌ **Unsupported** — intentionally unsupported for the stated release.
- N/A — not applicable.

| Target | Architecture | Compile | Shader | Pipeline | Runtime | Device | Performance | Public status |
|---|---|---|---|---|---|---|---|---|
| Windows / D3D12 | ✅ Compatible | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | Not qualified |
| Linux / Vulkan | ✅ Compatible | ✅ cargo test --workspace（本机） | ✅ naga 全量校验 + 真实管线创建（golden/budget 测试） | ✅ 全 class pipeline 创建（golden/budget 测试） | ✅ golden 捕获全绿（本机） | ✅ RTX 5090（本机 baseline，provenance 记录于各 `*.golden.json`） | 🧪 本地 CPU sweep 与单视图 GPU 时间戳；绝对预算未定义 | Experimental（本机证据，single-device） |
| Linux / GLES | ✅ Compatible | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | Not qualified |
| macOS / Metal | ✅ Compatible | ✅ cargo test --workspace（M4 Pro 本机） | ✅ naga 校验 + 真实管线创建（golden/budget 测试） | ✅ 全 class pipeline 创建（golden/budget 测试） | ✅ golden 捕获全绿（macos-metal overlay，5 图） | ✅ Apple M4 Pro（单一设备；overlay 验收记录） | ⚪ Not measured | Experimental（single device: Apple M4 Pro, Metal, macOS 27.0） |
| iOS / Metal | ✅ Compatible | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | Not qualified |
| Android / Vulkan | ✅ Compatible | ✅ ARM64 smoke, local Bevy fork | 🧪 SwiftShader + MoltenVK smoke | ✅ 全 18 目录场景（catalog lane） | 🧪 Procedural + static character, touch/resume | 🧪 API 35 emulator only | ⚪ Not measured | Experimental; hardware follow-up scoped below |
| Android / GLES | ✅ Compatible | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | Not qualified |
| Web / WebGPU | ✅ Compatible | ✅ standalone wasm32 | ✅ real browser compilation | ✅ Cel/Character/Hero + hull | ✅ static asset capture | ✅ Linux/Chrome 153 (envelope) + macOS/Chrome 153 (exact) | ⚪ Not measured | Experimental; per-version qualification (ADR-0021), scoped below |
| Web / WebGL2 | Outside current commitment | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | No support promise |
| Consoles | Port-ready architecture only | N/A | N/A | N/A | N/A | N/A | N/A | Port-ready |

## Public status definitions

**Supported** requires compile + shader + backend pipeline + representative real runtime render + showcase smoke + release regression evidence.

**Experimental** means architecture/compile/pipeline work exists but runtime/device/performance qualification is incomplete.

**Port-ready** only means there is no known architectural lock-in; it is not a shipping-support claim.

## Evidence records

When a cell becomes `Validated`, link or record an local evidence ID (device record, Golden run, benchmark report) in the release qualification record.

## Compatibility statement（provisional pending ADR-0027）

- **Bevy**: `=0.19.1`（pinned；V1 开发期锁定 0.19.x，升级走 ADR-0027 正式流程）；
- **Rust toolchain**: stable 1.98.x（`rust-toolchain.toml` 锁定）；
- **实测设备**：Linux x86_64 / NVIDIA RTX 5090（driver 615.71.09）；Vulkan 原生回归及以下 BrowserWebGpu 记录均来自该机；macOS 27.0 / Apple M4 Pro（Metal）为 golden overlay 合格的第二实测设备（2026-09-23）；
- 其余目标一律按上表的 Not qualified / Experimental 处理。

## Android emulator local record, 2026-09-21

Evidence ID `android-smoke-20260921`, collected in the workbench's ignored
`.local/validation/android/` directory (`provenance.json`, `results.json`, build
logs, per-process logcat and PNG captures). The reproducible application and
commands are in [Contributing](../../CONTRIBUTING.md#android-vulkan-smoke).

Host: macOS 27.0, Apple M4 Pro, 24 GB. Android Emulator 37.1.11,
`Tenacity_API35`, Android 15/API 35, ARM64, 4 KiB pages, 1080×2400 display.
Rust 1.98.0, NDK 30.0.16248370, Temurin 26.0.2.1, Gradle 9.4.1 and AGP 9.2.1
built and installed the debug APK with a release-optimized Rust library.
APK SHA-256: `301ea5d7c9a6612ec2758e2c22ea0012b48e3fb6272eeccafbe474bb54586a37`.
The source was Stylized `936263598de53000aef76ef6840ac974bea9f6af` plus the
uncommitted Android application, using workbench Bevy
`969ac3a3e98dae1fa4a56ecbba3a0e7f53be61b6`. The local source/lock hashes are in
the provenance record; this is not standalone registry-consumer qualification.

- **Host Vulkan: FAIL.** Actual adapter Apple M4 Pro, MoltenVK 1.4.0.
  MSL compilation rejects conversion from `float4[5]` to
  `spvUnsafeArray<float4, 5>`, followed by pipeline initialization failure and
  device loss. It reproduced after fixing the app's initial missing tonemapping
  LUT configuration. The affected shader has not been minimized to a standalone
  reproducer; this is a hardware-translation blocker, not proof every Android
  GPU fails or that Vulkan validation alone proves the shaders portable.
- **SwiftShader Vulkan: scoped smoke PASS.** The identical APK, with actual
  adapter SwiftShader Device (LLVM 10.0.0), renders the standard cube and all
  three procedural surface classes with three hull outlines. Portable resolved
  without tier reduction. Touch rotates the cube; Home and hot resume retain
  the same process and rendered scene, with Suspended/Running markers and more
  than 1,200 update frames. The captured process log contains no shader validation,
  device-loss or panic errors. Each sphere's region contains 2,366 outline-color
  pixels. Two stable captures after resume are identical in the application ROI
  `(0,160)-(1080,2240)`; this is repeatability, not accepted Golden comparison.

Bevy emitted a dependency dead-code warning; Gradle reported deprecated API use.
Software mode also logs a platform rendernode diagnostic and winit lifecycle/
inset TODO warnings. They did not prevent this scoped run. No real device,
16 KiB runtime, GLES, advanced textured material effects, skin/morph, post,
prepass, upstream visual parity or performance budget is qualified. Software
rendering timings must not be used as mobile hardware performance evidence.

### Static character extension

Evidence ID `android-character-20260921`, workbench local evidence directory
`.local/validation/android-character/`. The source-hash-verified Odette pack and
Gacha 3.6.9 were rebuilt with Blender 5.2.2 LTS and the unchanged
`genshin-static-emission-v1` recipe. All asset-build stages passed; the authoring
audit has no missing images, invalid constraints or drivers. Build success is
not a new Blender/runtime parity result.

The `character` Android feature renders the actual compiled GLB and adopts its
NPR sidecar/view. APK SHA-256:
`abda1a1d9fa31eef2e48698dec5c64c334102c55eb823edf82dc7d2b7c862748`.
All 56 packaged asset files match the compiled input bytes. On the same API 35
SwiftShader emulator, the app adopts 10 primitives, 9 exact material names and
41 material-image references. Seven stable hull entities match the seven
outlined primitives declared by this candidate's GLB/sidecar. Full-body front,
touch-rotated and resumed screenshots were visually inspected at 2400×1080;
touch and Home/hot-resume preserve the same process. Captured application logs
contain no shader validation, device-loss, panic or material-adoption errors.
Release compilation and Android Clippy pass for this feature.

This is a scoped static-character smoke result. The GLB has no skins or animation
clips, and the recipe excludes `Eff_001`/`Eff_002`. This extension does not resolve
the hardware MoltenVK blocker or qualify animation, source/runtime pixel parity,
real devices or mobile performance.

### Hardware Vulkan follow-up

Evidence ID `android-vulkan-20260921`, workbench local directory
`.local/validation/android-vulkan/`. The preceding failures remain the results
of their original APKs. On the same Emulator 37.1.11 / MoltenVK 1.4.0 / Apple M4
Pro stack, the original procedural APK reproduced the compile failure. Using
[MoltenVK shader dumps](https://github.com/KhronosGroup/MoltenVK/blob/main/Docs/MoltenVK_Configuration_Parameters.md#mvk_config_shader_dump_dir)
identified `portable_outline.wgsl`'s fragment entry point: dynamic indexing of
the `let` structure's palette extracted a whole array, producing the invalid
MSL assignment `spvUnsafeArray<float4, 5> _299 = _229._m1`. An addressable local
`var` instead emits direct `_213._m1[_219]` indexing. The fix preserves the
uniform ABI, authored class selection, gamma, extrusion and exposure arithmetic;
no material, outline, driver or dependency version was replaced.

Both rebuilt APKs render on the actual Apple M4 Pro Vulkan adapter. Procedural
Standard/Cel/Character/Hero surfaces retain three hull entities. The static
character retains 10 primitives, nine material names, 41 image references and
seven hull entities; all 56 packaged asset files match the preceding character
APK. The character's touch rotation and Home/hot-resume succeed in the same
process, with Suspended/Running markers and more than 1,200 update frames.
Per-process logs contain no shader compilation, device-loss or panic errors;
front and resumed screenshots were inspected at 1080×2400, MSAA off.

Procedural APK SHA-256:
`61ec05ac881eb351a9be18403566c5b82dbebda6f9a01ab500d8d652a1946f9b`.
Character APK SHA-256:
`a83ffbf742b7884105a2c38832997a40ab80d83f0d37a1ead008922770e0fed5`.
Naga shader composition/validation and documentation checks pass. Native macOS
GPU run `5c0ee4a160f14ed09555a9c1e5bec4e2` executes all eight registered cases in
the selected `outline_advanced`, `extended_skin` and `morph_skinning` suites:
six pass, including all five authored palette classes, disabled-class fallback,
complete skin/morph versus baked geometry and live outline width/deformation.
Two fail and remain unqualified: `outline_advanced` exceeds its accepted Golden
maximum (49 versus 4), and the external head-pose case hits Naga's native Metal
`_tmp` redefinition error. Both failures reproduce with the original shader;
the original/fixed hull-only, screen-only and hybrid captures are pixel-identical
on this machine. No Golden images or tolerances were changed. This is partial
suite coverage, not a passed full GPU lane.

2026-09-22 update: the Naga `_tmp` redefinition is fixed at the WGSL level
(commit `c177382`; struct-typed debug returns isolated into block scope).
`morph_skinning` (5/5), `hero_precision`, `profile_lighting` and
`sampling_camera` now pass on Metal. Remaining pre-existing macOS-host
failures: `outline_advanced`, `golden_cel`, `golden_character` are Metal
golden-delta cases (cross-device Golden comparison on Metal was never
qualified, docs/reference/08 §1), and `asset_adoption` requires an explicit
local `STYLIZED_TEST_ASSET` input that was not provisioned in this run.
macOS stays Not qualified above.

2026-09-23 root-cause of the three Metal golden deltas (diff-map evidence):
all three decompose into thousands of ±1 cross-vendor rounding pixels plus
1–2 outlier pixels at numerically hypersensitive boundaries, and none is a
fixable renderer defect. `golden_cel` sphere_outline: 992 px at delta 1 plus
one rim pixel at delta 6 (outline-silhouette coverage rounding).
`golden_character` face_sdf_instances: 9059 px at delta 1 plus two pixels at
delta 33 on the SDF threshold boundary of the high-threshold sphere, where
the steep smoothstep turns sub-ulp sampler/interpolation rounding into a
lit-vs-shadow flip. `outline_advanced`: screen-edge DETECTION was verified
correct on Metal (a direct prepass probe shows correct reverse-Z depth and
correct normals, and the edge map contains every baseline edge at the same
positions); the residual differences are sparse sub-pixel coverage/raster
rounding along edge lines (hull_only: 477/478 px below delta 48, one at 49;
screen/hybrid outliers to 186 at isolated edge pixels). No shader change can
remove these without altering the accepted Linux/Vulkan baselines, so the
suites keep failing on Metal exactly as before; the qualified Golden
comparison host remains Linux/Vulkan (docs/reference/08 §1).

2026-09-23 macOS qualification record (owner decision 2026-09-23): macOS/Metal
moves from Not qualified to **Experimental (single device: Apple M4 Pro, Metal,
macOS 27.0)**. The golden baseline host stays Linux/Vulkan; macOS qualifies
through the platform overlay mechanism
([fixtures/README.md](../../fixtures/README.md#golden-platform-overlays),
docs/reference/08 §1). Five overlay images (only the cases whose deltas are the
root-caused platform truth above) are captured on the device and accepted in
[fixtures/golden/macos-metal.acceptance.md](../../fixtures/golden/macos-metal.acceptance.md):
`cel-portable/sphere_outline`, `character-portable/face_sdf_instances`,
`outline-advanced/{hull_only,screen_only,hybrid}` (the earlier "three deltas"
named suites; outline-advanced contributes three cases because its lane runs
stopped at the first assertion). With the overlay, the local `gpu --required`
lane passes 41/41 on the M4 Pro (the lane at acceptance); removing the overlay
returns exactly the three suite failures (negative proof). The lane later grew
with the showcase_golden suite (52/52 from 2026-09-23's coverage expansion,
59/59 since 2026-09-27's full-catalog baselines; the macOS run passes 59/59
with the overlay). Canonical baselines are byte-untouched and the
Linux lane is unaffected (overlay never consulted on the canonical host).
Non-goals unchanged: no multi-device claim, no performance qualification, no
"Supported" label.

On SwiftShader, cold-launched original/fixed APK pairs were each captured after
600 update frames at the same front pose, camera and 1080×2400 resolution.
Both procedural and static-character application ROIs `(0,160)-(1080,2240)`
are pixel-identical (maximum/mean channel delta 0, zero changed pixels).
These are same-backend A/B results, not hardware/software or Blender/runtime
parity.

This resolves the observed emulator translation blocker. Physical Android GPUs,
16 KiB runtime, animation, source/runtime parity and mobile performance remain
unqualified; the workbench source/lock limitations above still apply.

## WebGPU local record, 2026-09-21

Run `hardening-webgpu`, report `target/stylized/hardening-webgpu/report.json`:
Chrome 153.0.8010.52 on Wayland, Bevy 0.19.1, pinned Rust 1.98.0;
actual backend `BrowserWebGpu`, browser adapter NVIDIA/Blackwell. A standalone
registry consumer compiled outside the parent workbench; its lock SHA-256 is
`3f22dc2109a3c4b20c8edab59eb64908ca467e09dca29e643ce8c7be3a627c5c`.
The 512² fixture adopted all 10 compiled static-character primitives, created
8 hull outlines and rendered procedural Cel/Character/Hero samples. Browser
and native Vulkan RGBA readbacks were identical (maximum channel delta 0), with
no reported shader compilation or uncaptured GPU errors. The required flags and
reproducible local command are in [Contributing](../../CONTRIBUTING.md#browser-webgpu).

The successful adapter required a headed browser and explicit WebGPU/Vulkan flags;
headless discovery returned no adapter. This qualifies neither default browser
configuration nor another device/OS. WebGPU shadow/prepass, instance tables,
deformation, performance and mobile browsers remain untested. WebGL2 is outside
the owner's current support promise. Native screenshot parity does not establish
upstream Blender parity; that uses the separate strict character lane.

### WebGPU record addendum, 2026-09-22

Re-run on Chrome 153.0.8010.52 / Linux (NVIDIA 615.71.09, hardware Blackwell
adapter) with the working X11/XWayland flag set documented in
[Contributing](../../CONTRIBUTING.md#browser-webgpu): the lane completes
end-to-end (adapter, pipeline, fixture adoption, capture), but the exact-match
gate FAILS on 6 deterministic pixels / 18 channels (max channel delta 31/255,
hair specular highlights) from a Dawn (Tint) vs wgpu (naga) divergence. The same
fixture is pixel-exact on macOS Chrome 153 (Dawn/Metal vs wgpu/Metal). The
2026-09-21 "identical" record therefore no longer holds on the current browser;
a version-qualification or tolerance decision stays open in ADR-0021, which
remains a V1 Blocker. No row above changes; this addendum makes no new support
claim.

**Resolution (2026-09-22, later the same day):** ADR-0021 is Accepted with
per-browser-version qualification; the browser-vs-native gate now prefers exact
equality and otherwise applies the compiler-divergence envelope of
[reference/08 §5](../reference/08-golden-tolerance-and-cross-layer-criteria.md)
(≤13 differing pixels, ≤32/255 max channel delta, every differing pixel
tier2-edge-confined). The recorded Linux Chrome 153 divergence (6 px / 18 ch /
max 31) passes the envelope; the macOS Chrome 153 run remains exact. This is
still not a general browser-support claim: other browsers, versions, drivers
and feature scope are unmeasured until individually recorded.

### Browser showcase record, 2026-09-27

The `browser-showcase` lane ([Contributing](../../CONTRIBUTING.md#browser-showcase-lane))
executes the full 18-scene catalog in the browser through the CaseDriver host
(`apps/browser-showcase`, per-frame pumping, the shared showcase capture
profile) and judges per scene against the canonical showcase goldens
(browser vs same-machine native reference: exact preferred, the reference/08 §5
envelope is the only fallback). Canonical host (Chrome 153.0.8010.52 headless,
Dawn/Vulkan, NVIDIA Blackwell, driver 615.71.09): **18/18 PASS, all exact** —
the envelope was not needed. The macOS diagnostic run (Chrome, Dawn/Metal, M4
Pro) passes 16/18; `lod` (1 px, ±1, flat region) and `outline-advanced` (3 px
at frame row 0) fail the envelope's edge-confinement and are reported as
FAIL-with-evidence, not hidden; the macOS native-vs-golden leg passes all 18
(via the macos-metal overlay where accepted). Per-version scope unchanged: this
records Chrome 153 on the two measured hosts only.

### Android catalog record, 2026-09-28

The `android-catalog` lane ([Contributing](../../CONTRIBUTING.md#android-catalog-lane))
runs all 18 catalog scenes on the Tenacity_API35 emulator (API 35, arm64,
`-gpu host`; MoltenVK 1.4.0 forwarding to the Apple M4 Pro Metal) with a fresh
process per scene and per-scene structured reports. Result: **18/18 PASS**, two
batches byte-identical. Judgment is the golden tolerance against the canonical
showcase baselines, with the `android-vulkan` overlay for the four
platform-truth classes (`effects`, `face`, `stress`, `outline-advanced`; three
of them byte-identical to the accepted `macos-metal` images — same physical
GPU). The evidence is emulator evidence: the row's Experimental label refers to
this emulator identity; no physical-device, performance or thermal claim.

2026-09-28 courtyard browser lane note (Apple M4 Pro, Metal, Chrome 153.0.8010.53 Dawn):
two host-limited divergences recorded during the first browser-showcase run of
the courtyard capture matrix.

1. Vanilla Bevy PBR (`StandardMaterial` lit by the stylized key light) renders
   pure black under Dawn/Metal while rendering correctly under wgpu/Metal in
   the same process family. The reference GLB character exhibited this until
   the courtyard stage was moved to NPR materials (see the 2026-09-28 entry in
   `fixtures/golden/showcase.courtyard.acceptance.md`); the stylized NPR
   pipeline itself is byte-identical across browser/native for all procedural
   content. Root cause not investigated upstream (no minimal repro filed;
   recorded here as a host-limited upstream Bevy/Dawn discrepancy, not a
   defect in this repository's renderer).
2. Screen-space outline (`courtyard-outline-hybrid`) diverges systematically
   between Dawn (Tint) and wgpu/Metal (naga) on the same device: 2488
   edge-confined pixels with deltas up to 216 versus the browser envelope
   allowance of 13. Same class as the 2026-09-23 Metal screen-outline golden
   delta recorded above (compiler-sensitive edge detection); it is a platform
   variation, not a renderer defect. The capture fails the browser-vs-native
   envelope on this host; acceptance of a widened tolerance is an owner-level
   decision (per the macOS Experimental qualification precedent), not made
   here.
