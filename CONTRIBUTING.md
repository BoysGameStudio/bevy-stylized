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
Report results against [release criteria](docs/release/02-v1-release-gate.md) and
existing ADRs. Implementation and engineering independence do not accept open
visual, platform or publication decisions.

For documentation only, run `python3 tools/checks/check_docs.py` and verify moved
anchors, command entrypoints, generated inputs and source/schema references.

## Browser WebGPU

`python3 tools/dev.py check --lane webgpu --required --asset BUILD/compiled/character.glb --out target/stylized/NEW_WEBGPU`
builds one standalone consumer outside ancestor Cargo configuration, first for
wasm32 and then native Vulkan. It renders the same 512² character/class/outline
fixture in a disposable Chromium profile and a local native window, checks shader
and GPU errors, and compares actual RGBA readbacks exactly. A failed comparison
is retained, never rebaselined. This lane currently qualifies this Linux adapter;
it is not a portable browser support claim.

Prerequisites: pinned Rust `wasm32-unknown-unknown` target, cached standalone Cargo
dependencies, Node 22+, Pillow, a local display, Chromium and wasm-bindgen-cli
matching the workspace lock. Select executables with `STYLIZED_CHROME` and
`STYLIZED_WASM_BINDGEN`. `STYLIZED_CHROME_FLAGS` is a JSON array, recorded in the
report; the qualified Linux/Wayland invocation used
`["--ozone-platform=wayland","--enable-gpu","--enable-unsafe-webgpu","--use-angle=vulkan","--enable-features=Vulkan"]`.
Native X11 requires working `DISPLAY`/`XAUTHORITY`. The lane uses no personal
browser session, remote runner or WebGL2 fallback. See the
[support matrix](docs/platforms/SUPPORT_MATRIX.md) for the exact measured scope.
