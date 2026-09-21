# ADR-0004: NPR Instance Data Transport

- **Status:** V1 Blocker

## Context

Prototype MeshTag, GpuArrayBuffer, LUT/texture indirection and portable fallback without breaking batching.



## Evidence required

- authoritative API/spec/source references;
- prototype or benchmark where performance/compatibility is material;
- affected Golden / pipeline tests;
- effect on Portable baseline and fallback behavior.


## Revisit trigger

Revisit when the Bevy/wgpu baseline changes, measured evidence contradicts the decision, or a supported platform cannot satisfy the contract.

## Provisional lifecycle evidence, 2026-09-20

The StorageTable experiment now has a bounded dense snapshot lifecycle, explicit
admission failure, draw-binding revocation and material rebinding on layout
changes. The normative provisional contract is in
[Material / Instance / View](../rendering/02-material-instance-view.md#provisional-storagetable-lifecycle-2026-09-20).
CPU regressions live in `instance_table.rs`; the required integration suite adds
`table_churn_preserves_pixels` and retains batching measurements plus the Portable
binding gate. These are lifecycle findings, not acceptance of this ADR.

## Hero precision experiment and current choice, 2026-09-21

Keep the existing native Hero MeshTag path for the currently documented coarse
input contract: threshold step 1/254, maximum absolute threshold error 1/508;
profile/flags are eight-bit, face flip is one bit, non-default outline scale uses
seven-bit [0,2] quantization. The dense CPU sweep verifies monotonicity and the
threshold error bound. Required GPU tests now exercise independent head turns on
one shared Hero material and native outline width changes/removal without material
mutation. Exactly unit outline width preserves authored shell coordinates.

`tests/integration/tests/hero_precision.rs` derives a test-only fragment-input
prototype from the actual Hero shader, replacing only the threshold load with a
read-only table. It adds no production material or facade API. In the initial
512², softness 0.01 probe, both paths retain one opaque phase batch at 100 and
1000 instances; 119506 and 143405 pixels respectively differ between quantized and
full-precision inputs. The result demonstrates a precision boundary, not a broken
entity mapping or equal visual fidelity. It uses a 16-byte threshold probe record,
not the existing 48-byte complete Character record. CPU update measurements are
reported with their optimization flag; debug measurements cannot establish a
production speedup. Repeated release evidence is stored with its own run identity.

A Hero table shipping path is not selected by this experiment: no finer numeric
requirement has been declared for this hardening target, and the prototype does
not qualify table adoption, lifecycle, reload, fallback or browser behavior.
Existing Character StorageTable lifecycle tests continue to own those guarantees
for Character. HostMaterial's full-vector input is another bounded option, with
128 distinct records; it does not replace a 1000-unique-Hero table. A full-precision
Hero requirement would need that lifecycle generalization and affected whole-
pipeline budgets before promotion. GpuArrayBuffer, LUT and custom phases have no
demonstrated advantage for the selected coarse non-storage contract and are not
added merely to satisfy a prototype count.

ADR acceptance remains a maintainer decision about these precision/capability
boundaries. The experiment must not be described as a stable Hero table API.
