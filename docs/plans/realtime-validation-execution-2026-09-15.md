# Real-time validation execution — 2026-09-15

This stage delivers indexed camera/light scans, an optimized measurement path, and
targeted rendering-input fixes. It introduces no animation, expression, IK or
physics controller and accepts no new image or performance baseline.

## Results

The maintained [realtime lane](../showcase/04-realtime-validation.md) produced 720
sampled states / 1440 images across orbit, light rotation and face zoom. Repeated
RGBA outputs and all three closed-loop endpoints match exactly. Geometry remained
nonempty. Uniformly sampled contact sheets and additional face-light samples were
visually inspected; no obvious mesh loss or abnormal discontinuity was observed
in that reviewed subset. Full temporal quality is not inferred from repeatability.

Three optimized runs per resolution/mode measured the instrumented two-view render
graph. At 1024² per view / MSAA 4x, mean GPU spans were approximately 0.457 ms for
Beauty and 0.435 ms for Lighting factors. Device allocation was approximately
237.5 MiB, with 512 MiB reserved. These are measured candidate records, not presented
FPS, shipping budgets or total process VRAM. Stock diagnostic APIs did not provide
submitted draw-command counts; the report explicitly keeps them unavailable.

## Defects addressed and input boundary

- A new bounds test failed before the fix: outline replicas lacked updated
  producer AABBs when the mesh handle was unchanged. Changed and removed mesh-local
  source bounds now propagate, with regression coverage.
- `StylizedFaceInput` consumes a host-selected head transform and local basis.
  GPU tests prove shading changes and restores with external rotation while one
  material asset remains shared. Side selection, missing-head handling and removal
  are covered. Existing MeshTag precision and unsupported combinations are explicit.
- Inspector controls now order light updates before view synchronization, and scans
  check the actual managed light against the declared frame.
- Height-only focus biased closeups toward the hair. Face-SDF surface bounds now
  provide semantic focus before the approximate fallback.
- The first GPU timing probe was rejected because unfinished encoders were appended
  after render buffers. Ordered RenderContext bookends now surround render work,
  and a child-span sanity check prevents publishing that invalid interval again.

## Validation and evidence

166 CPU tests and 20 required GPU tests pass. The full isolated job rebuilds from
raw assets without networking, an existing asset cache, desktop session or sibling
project; it passes inspector tests, scan/measurement smoke checks and all six
latest-stable Gacha character comparisons. Workspace Clippy and docs checks pass.
Missing GPU is explicitly blocked.

[Machine-readable evidence](../quality/realtime-validation-2026-09-15.json) records
source identities, limits, rejected probes and exact outcomes. Open
`target/stylized/realtime-validated/index.html` for the measurement table and scan
review links. Draw-counter integration, exhaustive temporal review, full-precision
external face transport and a second real character are not claimed by this record.
