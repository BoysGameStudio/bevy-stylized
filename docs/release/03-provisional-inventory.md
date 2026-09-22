# Provisional implementation inventory

This is the current implementation map, maintained with the owning contracts. ADR
statuses remain owned by the [ADR index](../adr/ADR_INDEX.md); implementation or
test success does not accept an ADR. The complete [current validation workflow](../../CONTRIBUTING.md)
owns procedures; linked evidence retains its measured scope and limitations.

## Current implementation and remaining decisions

| Domain | Current code and evidence | Remaining decision / limitation |
|---|---|---|
| Material backend | [render plugin](../../crates/bevy-stylized-render/src/lib.rs), Cel/Character/Hero Material implementations | ADR-0001; no backend replacement or speed claim follows from this inventory |
| Whole-pipeline budgets | [budget tests](../../tests/integration/tests/interim_budget.rs) | ADR-0002/0003; interim limits and bindless policy remain provisional |
| Instance transport | [MeshTag](../../crates/bevy-stylized-render/src/instance.rs), [instance table](../../crates/bevy-stylized-render/src/instance_table.rs), [tests](../../tests/integration/tests/instance_table.rs) | ADR-0004; StorageTable exists, Character has the table material variant; it does not imply Hero full-precision face transport |
| Fallback and LOD | [capability](../../crates/bevy-stylized-core/src/capability.rs), [render LOD](../../crates/bevy-stylized-render/src/lod.rs) | Preserve class semantics and explicit tier/feature reductions; three-axis ADR acceptance does not settle every fallback algorithm |
| View, lighting, environment | [view](../../crates/bevy-stylized-render/src/view.rs), [post](../../crates/bevy-stylized-render/src/post.rs) | ADR-0009; authored mappings and environment limitations remain explicit |
| Outline | [replica synchronization](../../crates/bevy-stylized-render/src/outline.rs), [screen outline](../../crates/bevy-stylized-render/src/screen_outline.rs) | ADR-0008; updated/removed producer bounds and deformation compatibility are tested, not a general deformation solver |
| External face input | [adapter](../../crates/bevy-stylized-render/src/face_input.rs), [rendering-input contract](../showcase/04-realtime-validation.md) | External propagated head pose drives existing quantized instance input; no animation controller or general table support |
| Alpha/prepass/shadows | [semantic matrix](../../tests/integration/tests/semantics_matrix.rs) | ADR-0023 Accepted 2026-09-22; supported combinations stay explicitly bounded (masked screen-edge appearance and masked shadow casting remain unqualified) |
| Profile and texture interpretation | [Genshin mapping](../../crates/bevy-stylized-profile-genshin/src/mapping.rs), [Hero](../../crates/bevy-stylized-render/src/hero.rs) | ADR-0005/0006; current static qualification does not establish every family effect |
| glTF metadata and setup | [metadata](../../crates/bevy-stylized-gltf/src/metadata.rs), [setup](../../crates/bevy-stylized-gltf/src/setup.rs) | ADR-0020/0028; conflict rejection and pre-registration are implemented, final API decisions remain separate |
| Reference chain | [current authoring qualification](../reference/01-reference-authoring-workflow.md), [current source lock](../../assets/sources/gacha-authoring.json) | ADR-0007/0011/0012/0015; only qualified current static scope, no historical release lane |
| Platform/feature cost | [support matrix](../platforms/SUPPORT_MATRIX.md), [disabled cost tests](../../tests/integration/tests/disabled_feature_cost.rs) | ADR-0010/0021/0026/0027; no unmeasured backend becomes Supported |
| Distribution | [release checklist](V1_RELEASE_CHECKLIST.md), [provenance](../reference/05-license-and-provenance-boundary.md) | ADR-0017; publication remains disabled |

## Evidence interpretation

The current source lock selects the accepted authoring release. Current static
character parity is recorded separately from engineering independence, first-party
image regression and performance. Dated records are not rewritten when scope grows.
The task-level [report contract](../testing/02-task-results.md) prevents stale run
summaries from becoming new qualification evidence; it does not close visual ADRs.

## Completion sequence

This is the execution plan for framework hardening after the 2026-09-21 Gacha
qualification. It does not start game integration or publication. The default
`genshin-static-emission-v1` recipe, its source lock and unchanged strict image
criteria are the starting point. `Eff_001` and `Eff_002` are explicitly out of
scope. Character animation control, retargeting and physics remain host duties;
deformation tests below exercise renderer compatibility with first-party fixtures.

The owner's 2026-09-21 execution choices are WebGPU verification without a WebGL2
support promise, and measurement/relative-regression baselines without an absolute
CPU/GPU/memory budget. Absolute-budget qualification is therefore not a gate for
this hardening task; it remains a separate future release target decision.

All work and validation run locally. Serialize Cargo/GPU batches and set
`CARGO_INCREMENTAL=0`. Preserve the checkout lock and unrelated work. Store run
logs in `.local/` and captures/reports in `target/stylized`; keep contract updates
in their existing owning documents. A missing device, baseline or required metric
is BLOCKED, never a pass. Existing Golden images and thresholds are not changed
to resolve failures. Newly required baselines are candidates until explicitly
accepted. No commit, push or publishing is included in this plan.

### 0. Fix the candidate and evidence scope

Record the worktree/recipe/asset/standalone-lock hashes, toolchain, GPU/driver and
backend. Inventory the existing CPU/GPU/consumer suites and benchmark workloads,
distinguishing implemented tests, executed evidence and open design decisions.
Classify every release-checklist item as verified for a stated scope, missing an
implementation/test, missing a decision, or dependent on external hardware.
Do not treat an old checked box or an ADR status as a fresh measurement.

Before choosing transport or implementation budgets, record the required native
and browser targets, the available local devices/SDKs, and the release's intended
WebGPU/WebGL2 boundary. Discover hardware now rather than after implementation.
An unavailable target remains an external qualification dependency. A proposed
Web minimum is not an accepted policy, and an unresolved requirement must not be
silently converted into an exclusion.

Define required instance fields/precision, alpha/pass behavior and Portable
resource limits before implementing a new transport. Likewise, distinguish an
existing regression budget from a proposed absolute frame/memory target. If an
absolute target is not defined, measurements and regression checks can proceed,
but absolute performance qualification remains open; the measured result itself
must not be used to manufacture a passing budget.

Run a focused correctness probe before the large measurement sweep: authored
metal/ramp currently uses source controls and the per-instance extras flag, while
the material fallback clears generic Hero flags. Establish whether material-level
fallback actually disables the intended source branch. This is a source-inspection
finding, not a confirmed rendered failure. If reproduced, fix the defect and its
regression before measuring that fallback as a cheaper, correct variant.

Exit: a reproducible candidate identity and a mapping from each remaining item to
its owner, next action and required evidence. Keep dated measurements intact.

### Dependency and milestone rules

| Work package | Required predecessor | Completion evidence |
| --- | --- | --- |
| Local target/metric/semantic scope | Phase 0 inventory | Required cells, budget status, hardware gaps and diagnostic reproduction recorded |
| Current implementation baseline | Validated collectors and targeted correctness checks | Same-candidate timing/replay reports and review findings |
| Instance transport decision | Precision requirements and comparable prototype results | Selected design and rejected alternatives documented in ADR-0004 |
| Transport/fallback implementation | Transport decision and baseline | Registered CPU/GPU/lifecycle/budget tests and facade adoption coverage |
| Optimization | Correctness plus a measured bottleneck | Comparable before/after timing and unchanged visual contract |
| Final native qualification | Completed required native work and contracts | Final local gates plus one final-candidate isolated rebuild |
| Other backend qualification | Target scope plus matching SDK/device | Backend-specific runtime and regression records |

Contract reviews can advance as their evidence becomes available; they need not
wait for all optimization. Hardware-dependent cells do not stop unrelated local
work, but remain open in the overall plan. Failure is resolved in its owning work
package and invalidates its downstream evidence, not every unrelated prior check.
Run affected tests after each implementation change, not only at the final gate.
Register new mandatory GPU cases in `tests/suites.json` so ignored tests cannot
disappear from required coverage. Public feature/API changes also trigger the
affected standalone consumer check immediately. Keep measured experimental code
separate until the selected design has equivalent correctness coverage.

### 1. Establish trustworthy performance and temporal evidence

Owners: `tools/checks/realtime.py`, inspector validation, `labs/benchmarks`,
`labs/regression`, and the existing performance targets/testing contracts.

1. Audit metric names and collection boundaries before optimizing. The existing
   benchmark/sweep `App::update` wall times are CPU/process timings, not GPU
   timestamps. Keep those histories separate from the inspector's instrumented
   render-graph GPU span. Preserve allocator allocated/reserved bytes as allocator
   accounting, not total process VRAM. Keep draw-command counts unavailable until
   an actual submission counter exists; mesh/entity counts are not a substitute.
2. Give required benchmark runs the standard PASS/FAIL/BLOCKED report contract.
   The current `regression-lab` CLI can exit successfully when no GPU is found;
   required automation must reject that outcome. Check missing GPU, invalid or
   insufficient timestamp samples, missing baseline cells and mismatched devices.
   Explicitly inventory required cells so missing current/baseline cases, duplicate
   IDs, a Golden-required cell without Golden evidence, and changed workload or
   measurement metadata cannot produce PASS. Record build optimization/features,
   fixture/asset identity, driver/backend, resolution, view count, MSAA, requested
   and resolved tier, and timing/allocator metric definitions in comparability checks.
   This required benchmark adapter is planned work, not an existing CLI promise.
3. Run the optimized default character through the current `realtime` lane:
   512²/1024² per view, Beauty/Lighting, three repetitions, 60 warm-up updates and
   600 measured updates. Retain raw samples, mean/p95, actual sample count and
   measurement scope. Pause only task-owned competing GPU previews during timing
   and restore them afterward. Record other visible GPU load and power conditions.
4. Capture 120 samples each of orbit, light rotation and zoom, with two repeats.
   Review full frames plus face, hair, metal and outline regions, especially
   silhouette crossings, grazing highlights and toon thresholds. Repeat equality
   and loop closure are machine checks; absence of shimmer/pop requires visual
   review. Normal deliberate toon transitions are not automatically defects.
5. Complete the workload matrix already required by `docs/performance/targets.md`:
   1/3 Hero, 32 Character, 100/500/1000 Cel@Portable, Hero with skin+morph, outline,
   and full post. Reuse existing scene builders and add missing compositions.
   Keep the real static character workload distinct from procedural Hero fixtures.
   Include an isolated game-style single view so two-view inspector overhead is
   not presented as the cost of rendering one character.
   Each required workload needs a stable ID and explicit class/tier, resolution,
   view/pass settings, scene builder, measurement scope and visual oracle. Collect
   new workload evidence under fresh candidate paths. Existing semantic/numeric
   fixtures can validate their invariants while new Golden acceptance remains a
   separate decision; do not auto-create accepted images to unblock collection.
   The single-view timestamp path is an instrumentation deliverable to implement
   and verify, not an already available inspector mode.
6. Register target-specific budgets and comparability conditions before using them
   as new gates. Preserve the existing +10% sweep gate in its original metric
   domain. New GPU/memory gates need their own recorded target and noise evidence;
   do not invent universal millisecond budgets or retune thresholds after a fail.

Collection acceptance requires release optimization, declared readiness/warm-up,
finite positive timings, and at least the existing 90% valid GPU-sample requirement
per measured run. Keep all three repetitions visible, including p95, rather than
reporting only their pooled mean or the best run. When repeat variation prevents a
stable before/after conclusion, diagnose contention/clock/temperature conditions
and rerun under controlled conditions; otherwise record INCONCLUSIVE. Do not
discard slow samples or widen a gate simply to resolve that outcome. Attribute
claims only to the collected span: update wall time is not exclusive CPU render
cost, and allocator bytes are not a proven total-VRAM peak or a leak diagnosis.

Exit: complete required measurement coverage on the local target, explicit temporal
review findings, no missing samples silently accepted, and a prioritized list of
measured bottlenecks. Fix reproducible visual defects before calling the candidate
visually qualified. A valid measurement report alone is not performance acceptance.

### 2. Complete instance transport, sharing and fallback semantics

Owners: render instance/instance-table/face-input/material code, facade adoption,
`tests/integration`, Material/Instance/View and the NPR semantic contract.

1. Measure MeshTag quantization against the required face thresholds and rendered
   transitions, then compare a Hero table prototype with the existing path under
   the same precision/batching/cost requirements. Consider the other ADR-0004
   candidates only where they address an unmet requirement. Record the transport
   decision before committing to a permanent Hero table API. Higher precision
   should fix a demonstrated contract gap, not become an unmeasured feature quota.
   If a table path is selected, generalize the existing Character-specific binding,
   suspension/restoration and rebind lifecycle to Hero; share the allocator and
   record layout policy rather than duplicating them. If MeshTag already meets
   the declared target, document its measured error bound and the reason to retain
   it. An unmet mandatory precision requirement remains a blocker. Either path
   must preserve shared materials without per-frame asset mutation and define
   explicit overrides, admission failure and restoration precedence.
2. Retain the non-storage MeshTag path for the declared Portable contract. Make
   its quantization and supported fields visible. Never select a storage-dependent
   variant on a device lacking that capability; unsupported requirements must have
   a deterministic supported fallback or an explicit error.
3. Verify 100/1000 same-mesh/same-material instances with distinct NPR values,
   including two characters turning their heads independently. Exercise creation,
   deletion, compaction, delayed upload, capacity failure/re-admission, material
   reload, binding removal and scene teardown. Preserve batching and resource
   sharing; use measured batch/draw evidence rather than material-count inference.
   Check the facade's descriptor-to-material selection and the external face-input
   update path as well as standalone material fixtures. Cover host/native ownership
   conflicts and explicitly unsupported combinations. Verify main/outline/prepass/
   shadow paths only where declared supported; retain reference truth independent
   of the path being tested. Compare lifecycle memory with the live entity set over
   repeated churn; distinguish retained allocator capacity from a monotonic leak.
4. Audit every source-specific shading branch against both material fallback and
   per-instance LOD. In particular, check authored metal/ramp/normal branches whose
   enable inputs differ from the generic Hero flags. A logical feature disable
   does not prove that its samples or fixed bindings disappeared.
5. Test the final view+mesh+instance+material pipeline resources for each supported
   class/tier. Verify ramp ordering, class IDs, face-side choice, required outlines,
   alpha/culling, palette relationships, finite LDR output and truthful diagnostics.
   Cover rapid tier/LOD changes and skin/morph fixtures, while preserving the
   default character export's explicit static-outline limitation.

Exit: the selected Hero input transport and Portable fallback meet the declared
precision and capability requirements and are behaviorally tested; no instance
data crosses entities; teardown/reload stays bounded; required visual invariants
and actual pipeline budgets pass. Document any remaining precision limitation
against the selected tier rather than silently claiming equal fidelity everywhere.

### 3. Optimize only measured bottlenecks

Use phase 1 profiles and phase 2 instance/resource evidence to choose bounded
changes: redundant extraction/uploads, bind-group recreation, unintended material
duplication, outline pass cost or unnecessary texture work. Keep the existing
material backend unless a measured prototype demonstrates a reason to replace it.

For each change, preserve the same device, workload, resolution, visual contract
and source asset for before/after comparisons. Record CPU/GPU/memory/resource
deltas and run the affected semantic/Golden checks. An improvement smaller than
run-to-run variation is inconclusive. If no useful improvement is established,
retain the simpler implementation and record the finding; optimization is not a
quota of code rewrites. Changes to accepted budgets or visual baselines require
their own explicit decision, never a routine performance fix.

Exit: confirmed defects in required cells are fixed, required budgets pass, and all
claimed speedups have reproducible measurements without a visual regression.
If a mandatory target cannot pass, retain FAIL/BLOCKED and the concrete finding;
do not shrink its scope or redefine the budget to mark this work package complete.

### 4. Finish API contracts, feature isolation and design decisions

Audit actual manifests and callers before editing stale feature documentation.
Exercise existing default/minimal/core-only/pure-glTF/profile consumers, plus
currently declared inspector, schema and shadow-casting combinations where owned.
Add missing meaningful combinations to local tooling instead of testing features
that no longer exist. Verify disabled features do not allocate/upload/render work
outside their declared contract.

Review facade setup, glTF registration order, required `_NPR_*` attributes,
extras/sidecar conflict handling, material rebinding, alpha/prepass/shadow/outline
semantics, capability lifecycle and failure messages. Keep renderer internals and
character-specific policy out of the stable facade. Schema changes must update
generated schemas, producers, consumers and release notes together.

Close decisions in evidence order:

- Toolchain/reference/export/init: ADR-0007/0011/0012/0013/0014/0015/0028.
- Instance/material/resource/semantics: ADR-0001/0002/0003/0004/0005/0006/0008/0009/
  0020/0023/0026, after phases 1–3 establish their evidence.
- Platform/Web policy: ADR-0010/0021 in conjunction with phase 5.
- Provenance/distribution: prepare the factual source inventory for ADR-0017;
  engineering tests do not determine licensing or authorize publication.

Each closed ADR needs an actual Decision, linked Evidence, Consequences, supported
scope and revisit trigger. Do not bulk-change Proposed/Blocker to Accepted. Update
the ADR index, traceability, support matrix and release checklist consistently.

Exit: local consumer/feature checks pass; current public contracts match code;
eligible decisions are complete and remaining blockers name their exact missing
evidence or external decision.

### 5. Qualify platforms using real backend evidence

Start with Linux/Vulkan on the available device. Enumerate available adapters and
browser support before attempting local Linux/GLES and browser WebGPU checks.
Use real shader compilation, specialized pipeline creation, representative renders,
showcase smoke and regression checks; forced capability limits on Vulkan are useful
fallback tests but are not GLES/WebGPU runtime evidence.

Execute the Web scope established in phase 0 and close ADR-0021 with the resulting
policy and evidence: a WebGPU-only minimum and WebGL2 support are separate choices.
If WebGL2 is selected, implement and validate the non-storage path,
actual WebGL2 resource budgets and browser renders. Do not silently add that promise
or remove it solely to obtain green results.

Windows/D3D12, macOS/Metal, iOS/Metal and Android/Vulkan/GLES require their respective
local SDKs and real devices. Prepare reproducible local commands and evidence
records now; execute on those machines when available. Device/driver/OS, build,
class/tier resolution, images and timing scopes belong in each result. Compile-only
or missing-device cells remain Experimental/Not qualified/BLOCKED as appropriate.
Mobile targets additionally need their declared sustained thermal/power interval.
No remote CI runner is used for any target.

Exit: every claimed Supported cell has its complete evidence chain. Unavailable
hardware stays an explicit external dependency; finishing the Linux work does not
complete or silently waive those cells.

### 6. Run the final candidate and deliver the evidence

After source changes settle, run the affected local CPU, schema, consumer, inspector,
registered GPU, declared asset, strict character-reference and performance gates.
Then execute the default empty-cache/offline isolation rebuild once for that final
candidate. Repeat only checks invalidated by subsequent changes. Preserve the
standalone lock identity and never copy a workbench-only lock into publication.

Keep collection, correctness, performance-regression, absolute-budget and backend
qualification as separate result fields. Add a machine-readable required-item
summary under the existing run-output tree: each item records its scope, evidence
identity and PASS/FAIL/BLOCKED state. Optional measurements may be UNAVAILABLE or
INCONCLUSIVE, but cannot satisfy a mandatory item. The final result must reject
missing/stale/mismatched records and retain any required external dependency as
BLOCKED. It must not aggregate only the passing subset into an overall PASS.

New-workload baseline candidates, undefined absolute budgets and unavailable
hardware are explicit dependency records rather than requests for routine stage
confirmation. Continue all independent authorized work, and report exactly which
remaining evidence or target decision prevents full completion. Do not silently
mark a new Golden or performance baseline accepted as part of this execution.

Deliver the current default build/preview commands, actual pass/fail/block statuses,
reviewed captures, before/after performance records, completed contracts/ADRs and
remaining hardware or distribution dependencies. Local completion means the
available target and selected feature scope pass; all-platform or release completion
requires the remaining platform and governance gates as well. The two excluded
Aura effects never re-enter this completion checklist implicitly.

## Local execution disposition, 2026-09-21

The scoped source/evidence summary is generated at
`target/stylized/hardening-completion.json`. It must retain missing decisions and
external qualification dependencies; successful local checks are not an overall
V1 release acceptance. No game integration, commit, push or remote CI was performed.

| Work package | Executed local result | Remaining boundary |
| --- | --- | --- |
| Correctness | Source-specific Hero fallback, browser sampling uniformity, high-level post lifecycle, native instance outline scale and masked prepass defects repaired with registered regressions | Scoped behavior; not every upstream effect |
| Native checks | `hardening-final-cpu`: 205 Rust tests, 53 Python tests; workspace clippy; inspector unit/lint; seven standalone feature consumers | Optional shadow feature compile is not full runtime shadow qualification |
| GPU / assets | `hardening-final-gpu`: 41 registered cases plus synthetic adoption; declared asset adoption/reload; six strict default-character cases; inspector reload | Existing images and tolerances retained except the two explicitly approved separated-crowd images |
| Performance | Nine single-view workloads × three runs; real character two-view scans/measurements; 42-cell CPU sweep with pipeline readiness | Candidates collected; a new accepted v2 timing baseline and any GPU regression gate remain separate decisions; no absolute budget defined |
| Instance inputs | Dense quantization bound, two independent heads, native outline width, existing Character table lifecycle/batching; repeated release Hero table-input prototype | Current Hero MeshTag contract remains coarse; prototype does not promote a full-precision Hero lifecycle API |
| Optimization | Fixed inactive/incorrect work before measuring; retained simpler transport | Measured tag/table differences are smaller than repeat variation; no speedup claim |
| Temporal review | Exact replay/loop equality and nonempty diagnostics across 1440 images; reviewed contact sheet and largest-change neighborhoods | Sampling review does not establish universal absence of subpixel shimmer; sidecar records the reviewed scope |
| WebGPU | Maintained standalone consumer; actual Chrome/Wayland/NVIDIA adapter; 512² native/browser pixels identical | Explicit flags required; other browsers, tables, prepass/shadows/deformation and Web performance not qualified |
| Isolation | `hardening-final-isolation`: PASS for engineering, image regression and all six strict static cases; raw sources rebuilt offline | Performance inside isolation is SMOKE_ONLY; optimized measurements are the separately identified runs |

The initial final-check source identity is
`7db783aea871d3dba9015124ebabd91900c86db400eef615480fbce903ea1f06`.
Only CPU sweep readiness and deterministic animation-clock preparation changed
subsequently, producing
`1a46bf8d383877b55aa1ccfb654f78da67e62b0b62db4097b4aa0f8155791d61`.
A complete input-hash reconstruction verifies that all other source inputs are
identical (`.local/stylized-hardening/source-transition.json`). The affected sweep
and clippy were rerun; the final isolated snapshot uses the latter identity.

### Remaining release decisions and external evidence

- ADR-0004: accept the explicit coarse Hero/experimental table boundary or declare
  a finer requirement and qualify the complete table lifecycle before promotion.
- ADR-0007/0011/0012/0013/0014/0015: the current source-locked static recipe has strict
  evidence; broader family coverage and export/reference policy freezing remain
  review decisions, not failures hidden by changing the recipe.
- ADR-0001/0002/0003/0005/0006/0008/0009/0020/0026/0028: existing implementation,
  resource/feature tests and current contracts provide review inputs; their
  proposed API/budget/ownership policies are not auto-accepted by green tests.
- ADR-0023 (Accepted 2026-09-22): optional runtime Hero shadow casting and the
  masked hull/edge policy landed with GPU evidence (`shadow_casting.rs`,
  `masked_outline.rs`); masked screen-edge appearance (wide boundary band) and
  masked shadow casting remain explicitly unqualified cells.
- ADR-0010/0021: ADR-0021 is Accepted (2026-09-22) — WebGPU-only baseline with
  per-browser-version qualification and the reference/08 §5 divergence envelope;
  Chrome 153 records exist for Linux (envelope) and macOS (exact). ADR-0010 stays
  open for release-wide platform policy. WebGL2 is not promised.
  Windows/D3D12, Metal, mobile devices and Linux/GLES have no equivalent local
  qualification. This environment has no `adb`, Xcode, Wine or GLES discovery
  tools, and its current wgpu feature graph does not build GL.
- ADR-0017: source/license/provenance approval and publication remain maintainer
  responsibilities. This work does not publish the package or third-party art.

These are explicit open items. They must not be replaced with an all-platform,
full-precision or release-ready PASS, and they do not require repeated permission
for already authorized local implementation and verification.
