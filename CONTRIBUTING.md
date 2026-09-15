# Contributing

Read the current architecture and the specification for the affected subsystem.
Use Python 3.11+ for project tooling and the checked-in Rust toolchain. A renderer
change needs rendering evidence; a documentation change needs document validation.

## Common commands

```sh
cargo fetch --locked # initial Rust dependency preparation; no character assets
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
python3 tools/dev.py check --lane cpu
python3 tools/dev.py check --lane inspector-cpu
python3 tools/dev.py check --lane consumers --fetch
python3 tools/dev.py check --lane gpu --required
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
workflow filters and documentation together. Never duplicate render shaders into
a second source tree. Keep historical source attribution even after retiring code.

## Isolation

On the qualified Linux machine, prepare the current source packs, then run:

```sh
python3 tools/dev.py assets prepare --pack genshin-odette
python3 tools/dev.py assets prepare --pack gacha-authoring
python3 tools/dev.py check --lane independence --required
```

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
