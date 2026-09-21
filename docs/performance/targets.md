# Performance Targets

> **Status:** Normative format; concrete budgets are release/device data, not architecture constants.

Architecture documents do not hard-code one universal millisecond target. Performance qualification is relative to a named device/profile and a fixed visual-quality contract.

## Required target record

Every performance target must record:

```text
Target ID
Release / commit
Device / GPU / driver / OS
Backend
Resolution / dynamic-resolution policy
Refresh target / frame-time budget
Scene / fixture
Surface Class
Execution Tier
CPU render budget
GPU render budget
Memory / VRAM budget
Sustained-duration requirement
Thermal / power condition where relevant
Reference / Golden quality contract
```

## Baseline workloads

V1 must at minimum maintain stable workloads for:

- 1 Hero;
- 3 Hero;
- 32 Character;
- 100 Cel/Portable-execution instances;
- 500 Cel/Portable-execution instances;
- 1000 Cel/Portable-execution instances;
- Hero + skin + morph;
- Hero + outline;
- Hero + full post.

## Example record shape

```text
Target ID: android-mid-portable-100-v1
Device: <qualified device>
Backend: Vulkan
Resolution: <release-defined>
Scene: 100 Cel surfaces
Execution Tier: Portable
GPU budget: <release-defined>
CPU render budget: <release-defined>
Memory budget: <release-defined>
Sustained duration: <release-defined>
```

Values remain `Not qualified` until measured on the named target.

## Comparison rule

A change may claim a performance improvement only against:

1. the same workload;
2. the same device/backend or a clearly isolated comparable environment;
3. the same visual-quality contract;
4. recorded before/after measurements.

Related ADR: ADR-0029.

## Local collection and regression (2026-09-21)

The owner requested measured candidate baselines and relative regression, with no
absolute CPU/GPU/memory or frame-rate budget. Absolute qualification is therefore
`NOT_DEFINED`, not a fabricated pass or a reason to stop collecting evidence.

```sh
python3 tools/dev.py check --lane performance --required --out target/stylized/NEW_SWEEP
python3 tools/dev.py check --lane performance --required --out target/stylized/NEW_COMPARISON --baseline ACCEPTED_SWEEP_V2.json
python3 tools/dev.py check --lane workloads --required --out target/stylized/NEW_WORKLOADS
```

The 42-cell sweep measures `app-update-wall-ms/v1`, retaining its existing +10%
mean regression gate. Schema v2 records release optimization, actual backend and
driver, collector/workload fingerprint, quality and required Golden identities.
Missing/duplicate cells, changed metadata and missing Golden evidence fail.
Historical v1 records lack these identities and cannot act as v2 baselines.
Golden capture goes to the new report directory; collection never writes accepted
fixtures. The two separated-crowd images were explicitly accepted on 2026-09-21;
their manifest records original/new hashes and repeated candidate captures.
That approval does not accept their timing measurements.

`workloads` covers all nine scenes above, three runs each, 512², one view, MSAA4,
Portable execution, 24 ready frames, 60 warm-up updates and 600 measured updates.
It records raw CPU update and render-graph GPU timestamp samples and requires at
least 540 distinct GPU samples. Named outline/post/deformation components are
checked for activation; screenshot readback happens after timing. Reports keep
each repetition, mean/p95, material counts and optional allocator allocated/reserved
bytes. Allocator accounting is not total VRAM; opaque phase batches are not submitted
GPU draw counts. Actual submission counts remain unavailable in this collector.

The procedural Hero fixture includes source metal/ramp, normal and emission inputs.
It is separate from the external character's two-view inspector workload. The
initial `hardening-workloads` run exposed an inactive post controller and is not
valid full-post coverage. `hardening-workloads-final` ran all 27 cells with that
defect fixed: mean GPU spans ranged roughly 0.151–0.250 ms on RTX 5090/Vulkan,
driver 615.71.09. Later final-candidate runs retain their own task identities.
Run-to-run variation reached roughly 11–12% in several cases, so small optimization
claims are inconclusive. No new GPU percentage gate or absolute budget is accepted
by these collections. Preserve raw repetitions rather than selecting the fastest.

The final runtime-source collection (`hardening-final-workloads`) retained all
27 cells; GPU means ranged 0.1500–0.2415 ms. The real default character's final
two-view inspector means were 0.4786/0.4038 ms at 512² and 0.5469/0.4435 ms at
1024² (Beauty/Lighting). These are render-graph spans, not presented frame rates.
The release Hero input probe found only 0–0.005 ms mean CPU differences between
tag and table within each pair, smaller than roughly 0.05–0.08 ms variation
between repetitions; no speedup or mandatory table promotion follows.

The final CPU sweep (`hardening-sweep-ready`) additionally waits for 24 consecutive
ready render frames before the unchanged 15 warm-up and 40 measured updates.
Its deterministic strip animation counter is then reset so the four accepted
Golden capture poses remain unchanged. The required sweep passes all 42 cells and
all four images. Its fixed historical camera can crop dense crowd fields; the
single-view workload matrix explicitly fits each complete field instead. These
workload identities are distinct and must not be compared as the same render cost.
