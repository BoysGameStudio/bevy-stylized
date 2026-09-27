# Contributing

Read the current architecture and the specification for the affected subsystem.
Use Python 3.11+ for project tooling and the checked-in Rust toolchain. A renderer
change needs rendering evidence; a documentation change needs document validation.

## Local validation policy

Run validation locally through the commands below. GitHub Actions is disabled;
do not add remote CI as a required step or re-enable workflows. Retired workflow
definitions are not active validation inputs. Current commands and contracts own
maintenance; obsolete plans and historical report copies are removed.

## Common commands

```sh
cargo fetch --locked # initial Rust dependency preparation; no character assets
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
python3 tools/dev.py check --lane cpu
python3 tools/dev.py check --lane inspector-cpu
python3 tools/dev.py check --lane consumers --fetch
python3 tools/dev.py check --lane gpu --required
python3 tools/dev.py check --lane performance --required --out target/stylized/NEW_SWEEP
python3 tools/dev.py check --lane workloads --required --out target/stylized/NEW_WORKLOADS
```

All CLI paths are relative to the explicit project root (or to the config file for
configured values). From another working directory:

```sh
python3 /path/to/bevy-stylized/tools/dev.py --root /path/to/bevy-stylized paths
```

Pure asset CLI and profile compilation:

```sh
cargo run -p bevy-stylized-tools -- validate --json ASSET.glb
cargo run -p bevy-stylized-tools --features profile-genshin -- compile --profile genshin --strict --out NEW_DIRECTORY ASSET.glb
cargo run -p bevy-stylized-tools --features schema --bin stylized-schema -- schemas/runtime --check
```

Strict compilation requires the matching `.genshin.json` handoff and complete
material coverage. Generic compilation can emit default descriptors; this does
not establish Genshin conversion. Outputs are staged and committed into a new
candidate directory; existing outputs are not overwritten.

## Evidence and baselines

Capture output belongs under `target/stylized`; third-party authoring and runtime
packs belong under `assets/local`. First-party accepted baselines belong under
`fixtures/golden`. Missing baselines fail validation. Capture, comparison and
acceptance are separate operations. Do not use `GOLDEN_REBASELINE=1` as a routine
way to repair a failing test.

Read `docs/reference/08-golden-tolerance-and-cross-layer-criteria.md` before changing
comparison rules. Preserve the source hashes, case identity, actual GPU/backend
and color/camera settings. A report-generation pass is not a visual pass.

## Dependency and path changes

Product normal/build dependencies cannot point to apps/labs/tests or another
workspace. The glTF `bevy-integration` feature owns engine registration; tools
use its pure metadata layer. The optional core `schema` feature adds Schemars for
generation only. Check an isolated consumer before claiming minimal features.

Directory moves update Cargo paths, embedded resources, includes, script imports,
tool task selectors and documentation together. Never duplicate render shaders into
a second source tree. Keep historical source attribution even after retiring code.

## Isolation

On the qualified Linux machine, prepare the current source packs, then run:

```sh
python3 tools/dev.py assets prepare --pack genshin-odette
python3 tools/dev.py assets prepare --pack gacha-authoring
python3 tools/dev.py check --lane independence --required
```

Both ordinary builds and isolation default to `genshin-static-emission-v1`, with
source outlines, metal/ramp, normals and emission; `Eff_001` and `Eff_002` are
excluded. To select a reduced-feature regression recipe, add for example
`--recipe assets/recipes/genshin-static-normals-v1.json`. Only repository recipes
under `assets/recipes` are accepted; generated assets and external recipe paths
cannot bypass the empty-cache rebuild. The selected recipe and hash appear in
the inner and outer qualification records.
The runner resolves the standalone lock offline in the disposable source copy
before locked builds; it preserves the checkout's workbench lock and records the
standalone hash. Cargo remains offline throughout the isolated job.

The Bubblewrap recipe creates a Git-free source copy with spaces in its path,
starts with an empty project asset cache, disables networking, mounts only verified
raw source packs and required system/toolchain/GPU resources, and rebuilds the
character and reference inputs. The current recipe locates the qualified Blender
installation under `/snap/blender/current`; this is an explicit platform adapter,
not a runtime library dependency. Results and log paths are reported under
`target/stylized`. Other platforms need equivalent isolation adapters before they
can claim the same qualification.

## Task results and required test identity

See [task result contract](docs/testing/02-task-results.md). Each dev task publishes
RUNNING before work and a PASS/FAIL/BLOCKED terminal report under a unique
`target/stylized/runs/RUN_ID` directory. Fixed-name reports are convenience copies;
automation must supply `--run-id` before the subcommand and read that run's report.
A missing report, wrong identity or RUNNING state is never qualification evidence.

Required GPU cases are selected from `tests/suites.json`. The runner checks each
binary's ignored-test inventory, then executes every exact case separately so a
missing resource cannot mask another case's assertion failure. All cases continue
to be accounted for; FAIL takes precedence over BLOCKED in a mixed result.

`inspector-cpu` runs the optional inspector library tests and all-target clippy.
It needs the X11/Wayland development libraries used by that feature, but no live
window, Blender or character pack. `consumers --fetch` explicitly permits Cargo
to prepare each independent consumer's dependencies before offline checks; omit
`--fetch` when dependencies are already cached. It never downloads character art.


### Standalone graph versus enclosing-workspace integration

`architecture` (also used by `cpu`) resolves the strict dependency graph in a
Git-free source snapshot outside ancestor Cargo configuration discovery. It retains
the source snapshot identity and the standalone resolved lock hash. Only that copy's
lock may change when registry dependencies replace enclosing-workspace patches.
The checkout lock is unchanged. This does not allow arbitrary external paths or
product-to-lab edges, and is not a runtime isolation claim.

An enclosing workspace validates its reviewed local overrides independently.
`independence` goes directly to the existing Bubblewrap execution; its inner CPU
lane performs the strict graph checks within the isolated source environment.
Missing raw inputs, toolchains or GPU resources remain BLOCKED. Architecture PASS
does not imply the full independence/visual/parity lanes ran.

## Work sequence

Identify the affected contract, profile/capability scope and source lock. Change
its owner and actual consumers together. Select affected local checks; rendering
or authoring changes additionally need the corresponding GPU/asset/character scope.
Report results against the owning quality/testing contracts
([validation](docs/testing/01-validation-golden-and-performance.md)) and existing ADRs. Implementation and engineering independence do not accept open
visual, platform or publication decisions.

For documentation only, run `python3 tools/checks/check_docs.py` and verify moved
anchors, command entrypoints, generated inputs and source/schema references.

## Android Vulkan smoke

`apps/android-smoke` is a procedural ARM64 application using the public renderer.
It displays a standard-material cube (top left), Cel (top right), Character
(bottom left), and Hero (bottom right), with three hull outlines. It uses Vulkan,
Portable defaults, MSAA off and explicit `Tonemapping::None` (no LUT assets).
Tap to rotate the samples. Logcat markers prefixed
`ANDROID_SMOKE` record the adapter, capability resolution, scene counts, update
progress, touch input and application lifecycle. Update progress is not GPU timing
or proof of correct pixels. No external character assets are required; advanced
texture effects, skin/morph, post, GLES and real-device performance are separate.
The app's code and dependencies are Android-only; native workspace checks compile
an empty library without adding a window-system backend or requiring character
assets, including when the `character` feature is selected.

Prerequisites: the pinned Rust 1.98.0 toolchain and its `aarch64-linux-android`
target, cargo-ndk, JDK 26, Android SDK platform 36, Build Tools 37.0.0 and NDK
30.0.16248370. Set `JAVA_HOME`, `ANDROID_HOME` and `ANDROID_NDK_HOME` to existing
installations. The wrapper pins Gradle 9.4.1; AGP is 9.2.1 and GameActivity 4.4.0
matches android-activity 0.6.1. Android Studio is optional for the CLI workflow.

From this repository root, prepare dependencies and review the lock as usual
(enclosing workbench source overrides produce a local-only lock), then build
serially:

```sh
rustup target add --toolchain 1.98.0 aarch64-linux-android
CARGO_INCREMENTAL=0 cargo ndk -t arm64-v8a -P 31 -o target/android-smoke/jniLibs build --release --locked -p stylized-android-smoke
cd apps/android-smoke/android
./gradlew --no-daemon :app:assembleDebug
adb -s emulator-5554 install -r app/build/outputs/apk/debug/app-debug.apk
adb -s emulator-5554 shell am start -W -n org.bevystylized.smoke/.MainActivity
```

Choose the serial reported by `adb devices -l`. Start an ARM64 API 31+ AVD with
`"$ANDROID_HOME/emulator/emulator" -avd YOUR_AVD -gpu host -no-snapshot`.
Capture the emulator log and check actual backend/driver, not just its configured
mode. Apple Silicon host Vulkan uses MoltenVK; a separate `-gpu swiftshader` run
can help distinguish host-driver translation failures. Never combine timings
from these modes. Preserve logcat, screenshots and device identity under `.local/`.
Check Home/relaunch and touch explicitly. Compare repeat screenshots at the same
pose; do not replace accepted Golden fixtures. Emulator evidence does not qualify
a physical phone's performance or thermal behavior. Qualification belongs in the
[support matrix](docs/platforms/SUPPORT_MATRIX.md).

The 2026-09-21 hardware follow-up resolves the MoltenVK 1.4.0 outline shader
translation failure; `-gpu host` renders both fixtures on Apple M4 Pro. Keep the
outline fragment's indexed palette in an addressable local `var`: changing it to
`let` causes this translator to emit an invalid array copy. For investigation,
set `MVK_CONFIG_SHADER_DUMP_DIR` to an absolute local evidence directory when
starting the emulator; preserve the generated SPIR-V/MSL and the failing APK.
Do not disable affected materials or outlines and count a reduced scene as the
full smoke fixture. Consult the support matrix for the exact tested scope.

### Compiled character on Android

The opt-in `character` app feature replaces the procedural samples with the
compiled static character. It registers NPR glTF attributes before loading,
adopts the exact sidecar material names, applies the authored view, and logs
`ANDROID_CHARACTER` adoption counts. Tap rotates the character around its up
axis. It does not add animation or physics.

Prepare/build the default character using the commands in
[the integration guide](docs/authoring/05-integration-guide.md#inputs-and-preparation).
Use the returned successful `BUILD/compiled` directory, not a partial build or
raw game GLB. From this repository root, keep the same exported directory for
both commands so the embedded descriptors and APK textures match:

```sh
export STYLIZED_ANDROID_CHARACTER_DIR="/absolute/path/to/BUILD/compiled"
CARGO_INCREMENTAL=0 cargo ndk -t arm64-v8a -P 31 -o target/android-smoke/jniLibs build --release --locked -p stylized-android-smoke --features character
cd apps/android-smoke/android
./gradlew --no-daemon :app:assembleDebug
adb -s emulator-5554 install -r app/build/outputs/apk/debug/app-debug.apk
adb -s emulator-5554 shell am start -W -n org.bevystylized.smoke/.MainActivity
```

The directory must contain `character.glb`, `character.npr.json`,
`character.view.json` and all referenced textures. Third-party character data is
packaged only into ignored local build outputs. To return to the procedural
fixture, unset `STYLIZED_ANDROID_CHARACTER_DIR`, rebuild without `--features
character`, then repackage/reinstall. Both modes use the same application ID.

## Browser WebGPU

`python3 tools/dev.py check --lane webgpu --required --asset BUILD/compiled/character.glb --out target/stylized/NEW_WEBGPU`
builds one standalone consumer outside ancestor Cargo configuration, first for
wasm32 and then native Vulkan. It renders the same 512² character/class/outline
fixture in a disposable Chromium profile and a local native window, checks shader
and GPU errors, and compares actual RGBA readbacks exactly. A failed comparison
is retained, never rebaselined. This lane currently qualifies this Linux adapter;
it is not a portable browser support claim.

## Browser showcase lane

`python3 tools/dev.py check --lane browser-showcase --out target/stylized/NEW_BROWSER_SHOWCASE`
runs every registered catalog scene in the browser through the
`apps/browser-showcase` host (CaseDriver per-frame pumping; the shared
`showcase::capture_profile_graph` capture contract) and judges per scene:
browser (Dawn/Tint) vs fresh same-machine native capture of the same host
binary — exact preferred, the reference/08 §5 envelope is the only fallback —
plus the native capture vs the platform's accepted showcase golden at the
golden tolerance (Linux: canonical `fixtures/golden/showcase/`; macOS: the
`macos-metal` overlay where accepted). Both judgments are reported per scene;
a scene failing either is a FAIL with evidence. No browser baselines are
created. Per-scene failures are isolated and all results are reported.

Prerequisites match the webgpu lane (same executables/env), except no compiled
character asset is required: the catalog scenes are procedural, and the
`reference` scene's first-party fixture is served from `fixtures/`.

Prerequisites: pinned Rust `wasm32-unknown-unknown` target, cached standalone Cargo
dependencies, Node 22+, Pillow, a local display, Chromium and wasm-bindgen-cli
matching the workspace lock. Select executables with `STYLIZED_CHROME` and
`STYLIZED_WASM_BINDGEN`. `STYLIZED_CHROME_FLAGS` is a JSON array, recorded in the
report; the qualified Linux/Wayland invocation used
`["--ozone-platform=wayland","--enable-gpu","--enable-unsafe-webgpu","--use-angle=vulkan","--enable-features=Vulkan"]`.
Native X11 requires working `DISPLAY`/`XAUTHORITY`. The lane uses no personal
browser session, remote runner or WebGL2 fallback. See the
[support matrix](docs/platforms/SUPPORT_MATRIX.md) for the exact measured scope.

Measured with Chrome 153.0.8010.52 (2026-09-22, NVIDIA 615.71.09):

- Chrome 153 rejects `--ozone-platform=wayland` combined with `--use-angle=vulkan`
  (`--ozone-platform=wayland is not compatible with Vulkan`). The working hardware
  invocation is
  `["--ozone-platform=x11","--enable-unsafe-webgpu","--enable-features=Vulkan","--use-angle=vulkan"]`
  on an XWayland `DISPLAY`. Wayland without the Vulkan pair falls back to
  SwiftShader (`adapter: google/swiftshader`) and fails the exact comparison.
- When launched from SSH or another session without a user bus, Chrome navigation
  stalls before any fetch (pages never load). Export the session bus, e.g.
  `DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus`, in addition to
  `DISPLAY`/`XAUTHORITY`/`XDG_RUNTIME_DIR`.
- With hardware WebGPU on this combination, Dawn (Tint) and wgpu 29 (naga)
  diverge deterministically on 6 pixels / 18 channels (max channel delta 31/255,
  hair specular highlights of the genshin-odette fixture); the exact-match gate
  therefore reports FAIL on Chrome 153. The failure is retained per policy;
  cross-layer tolerance changes follow
  [docs/reference/08](docs/reference/08-golden-tolerance-and-cross-layer-criteria.md).
