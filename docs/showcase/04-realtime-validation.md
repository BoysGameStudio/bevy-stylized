# Real-time rendering scans and measurement

The inspector supports two separate development modes. Neither accepts a Golden
baseline, qualifies a shipping frame budget or owns character animation.

```sh
python3 tools/dev.py check --lane realtime --required --asset BUILD/compiled/character.glb --out target/stylized/NEW_REALTIME_RUN
```

This builds the optimized inspector, captures an indexed scan, measures Beauty and
Lighting factors at 512² and 1024² per view in three runs each, and generates a
portable review page. Pillow is required for the review artifacts. GPU tasks and
Cargo builds must be serialized. New output directories are required.

## Scan contract

`inspect --scan --frames 120 --resolution 1024 --out NEW` executes full camera
orbit, full light azimuth rotation and face-targeted near/far zoom. Each path has
120 samples and is repeated twice, with a virtual step of 1/60 second. Both live
views are captured, yielding 720 sampled states / 1440 PNGs. The default modes are
Beauty and Lighting factors. The light transform is checked against the declared
recipe in the same update, not one frame later. Closed trajectories reuse exact
endpoint values to avoid floating-point trigonometric seam differences.

The scan uses normal MSAA 4x and no offline supersampling. Pipelined rendering is
disabled when present to make pose-to-readback association explicit. Readback and
PNG encoding affect wall-clock pacing; this pass is not a performance benchmark.
Requests precede the serial update and are drained with a bounded queue.

Required checks: each repeated frame matches exact RGBA bytes, each loop's endpoints
match, and the lighting diagnostic retains nonempty geometry. Adjacent-image deltas
and contact sheets aid inspection. They include expected motion and intentional
toon thresholds, so they cannot automatically prove the absence of shimmer or
popping. The review page keeps this distinction visible. Supplementary face crops
are inspection aids and never replace full-frame comparisons.

The focus preference is explicit `inspector-focus.json`, then the bounds of actual
face-SDF material surfaces, then the labeled height approximation. The fixed
Blender reference cameras remain unchanged.

## Performance contract

`inspect --measure --frames 600 --resolution 1024 --out NEW` uses no screenshot
readbacks or PNG encoding. Each mode first reaches pipeline/asset readiness, then
warms for 60 render updates and samples 600 updates while orbiting. Both cameras
use the same mode, so Beauty and Lighting comparisons have equal view counts.

Bevy RenderDiagnostics timestamp queries measure an ordered `inspector_frame`
render-graph span. Bookends use `RenderContext` so their command buffers surround
render work in submission order; merely appending two unfinished encoders is not
sufficient. Child-span timing is checked against the outer span. The span covers
the render graph, not every GPU operation in the process, presentation or CPU
logic. Diagnostic query/readback overhead remains enabled in both modes.

CPU values are wall time around `App::update` including inspector update work;
they are not GPU time or displayed FPS. GPU diagnostics arrive asynchronously:
repeated measurements are deduplicated, actual counts are reported, and at least
90% of the requested GPU samples must be present. GPU samples are not matched to
exact camera pose indices. Reports retain raw diagnostic values and summaries.

Memory comes from the wgpu device allocator: allocated and reserved bytes are
separate and are not total process VRAM. Stock Bevy's diagnostics do not provide
submitted draw-command counts. A draw-function dispatch can issue zero or multiple
GPU draws; primitive/mesh counts cannot substitute. `draw_calls` therefore remains
explicitly `UNAVAILABLE` in this measurement path. This is a metric coverage limit,
not a claim that the renderer submits zero draws.

Optimized-build status, device/driver, resolution, view count, MSAA, sample counts,
asset identity and instrumentation scope are recorded. Three repetitions expose
run-to-run variability. Outputs are measured candidate baselines; no performance
budget or accepted regression baseline is silently established.

## Rendering-input contracts added in this batch

`StylizedFaceInput` attaches to a rendered Hero face mesh and names an external
head entity plus forward/up vectors in that head's coordinates. The renderer reads
its propagated transform and updates only instance face threshold/side fields.
It does not move the head or alter animation, expression, rigging or physics state.
Material assets remain shared. Removing the binding restores prior face overrides;
missing head/material/basis data restores those values and exposes a
`StylizedFaceInputStatus` error rather than retaining stale overrides.

This adapter currently uses MeshTag transport with the existing threshold
quantization (1/254). Storage-table slots and explicit material-side overrides are
rejected; it does not claim full-precision dynamic Gacha parity. The host must
supply the correct local basis from known authored/rest-pose conventions (not
infer it from an arbitrary currently animated pose) and inspect binding status. Registration orders it
after transform propagation/material flag synchronization and before tag upload.
First-party CPU/GPU tests verify turn, restore, mirrored side selection, missing
head cleanup and preservation of one shared material asset.

Outline replicas now mirror producer-supplied mesh-local AABBs when they change without mesh
replacement. Removing producer bounds removes the mirrored bounds so normal
calculation can resume. Producers remain responsible for updating source bounds
when external deformation changes geometry. This repairs a demonstrated replica
synchronization defect; it does not implement a deformation or cloth solver.
