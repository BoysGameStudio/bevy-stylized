# Asset build attempts and recovery

`python3 tools/dev.py assets build --pack genshin-odette --blender blender` continues
to use the exact prepared source packs and static recipe. It prints the actual
build directory. A failed build can be retried with the same command and inputs;
no recipe edit, manual cache deletion or baseline acceptance is needed.

## Content identity and attempt identity

The input digest owns the content key. New work lives under
`assets/local/PACK/builds/KEY/attempts/ATTEMPT_ID`. Each attempt has a unique ID;
failed/interrupted directories remain untouched. A completed legacy build directly
under KEY remains readable. Completed attempts are reused only after manifest
schema/nonempty file-map validation, input equality, contained relative paths and
artifact hash verification. Corruption is a failure, not a
reason to quietly select another candidate.

`build-manifest.json` is the completion marker. It is written atomically only after
all build stages succeed. A temporary manifest left by termination does not count
as completion. If termination occurs after that marker but before the journal's
final update, verified artifacts can be reused; the earlier journal is not rewritten.
`attempt.json` is operational evidence and is excluded from artifact hashes.

## Stage evidence

Each attempt records source/compiler/tool input digests and an ordered stage list.
For each process stage, the journal records the command, input artifact hashes,
changed/new output hashes, removed output names, exit code, log path and status.
Environment variables are not dumped. The authoring setup uses its own copied
add-on and Blender configuration, preserving the original source pack and normal
preferences.

A running attempt or stage is not a success. Handled errors retain FAIL or BLOCKED
and partial outputs; uncatchable termination may leave RUNNING. Missing required
resources remain BLOCKED through the existing developer task contract. A new
attempt starts the complete workflow, rather than trusting partial Blender state.
There is no cross-stage resume engine.

The current `genshin-odette` path and historical opt-in `reference` path keep their
existing steps and scopes. Authoring/import, audit, capture, export, compile and
matched comparison preparation do not imply full rig/effect support. Build success,
asset integration and upstream parity require their separate validation lanes.

## Inspect and verify

Use the printed build directory's `attempt.json`, stage logs, `build-manifest.json`,
`authoring/audit.json`, `compiled/character.glb` and `character-matched/case.json`.
Keep these together when diagnosing a failure. First-party baseline acceptance
is still a separate explicit action and is never part of retry.
