# Documentation System Review

> **Status:** Current review of documentation-system governance after the v0.2 pre-freeze repair.

## Verified strengths

- Archive integrity is separated from normative authority.
- ADR IDs are defined only by `docs/adr/*.md`.
- `ADR_INDEX.md` is derived from actual ADR files.
- All 29 ADRs are represented in `TRACEABILITY.md`.
- Accepted ADRs contain explicit Decision, Evidence and Consequences.
- Split subsystem documents have independent H1 titles and normative status.
- Missing semantic/performance contracts are now present.
- Platform evidence uses explicit status semantics rather than anonymous placeholders.
- The historical Master Plan is visibly marked `ARCHIVED / NON-NORMATIVE`.
- A local documentation linter is part of the package.

## Architecture status

The documentation system is **Pre-Freeze Review**, not Architecture Freeze Candidate.

The reason is technical, not documentary: unresolved `V1 Blocker` ADRs still exist. V1 architecture may advance to Freeze Candidate only when those blockers are closed or explicitly moved out of V1 scope.

## Remaining work

The remaining high-priority work is to close technical decisions, not to restructure the documentation system:

- Runtime Material Backend;
- Whole-Pipeline Resource Budget;
- Instance Data Transport;
- Web Baseline Policy;
- Render Semantics Matrix;
- License / Provenance;
- glTF initialization and metadata-precedence contracts where still unresolved.

## Maintenance rule

Run:

```bash
python3 tools/checks/check_docs.py
```

for every documentation change and in CI.

A rendering-contract PR is incomplete if it changes code semantics without updating the affected subsystem spec, ADR/traceability record and release evidence obligation.
