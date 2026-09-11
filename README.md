# Bevy Stylized Architecture Documentation System

This package restructures the single 5000+ line Master Plan into a maintainable architecture system.


## Naming

- Project / repository / Cargo package: `bevy-stylized`
- Rust crate identifier: `bevy_stylized`
- Subpackages use kebab-case on disk/Cargo and snake_case in Rust.

## Reading order

For a new contributor:

1. `ARCHITECTURE.md`
2. `GLOSSARY.md`
3. `TRACEABILITY.md`
4. `docs/architecture/01-goals-scope-and-three-axis-model.md`
5. `docs/architecture/02-target-architecture.md`
6. `docs/rendering/02-material-instance-view.md`
7. Relevant subsystem document
8. Relevant ADR(s)
9. `docs/release/02-v1-release-gate.md`

## Documentation authority

- `ARCHITECTURE.md` is the index and invariant summary.
- Subsystem documents are normative for their own domain.
- ADRs record irreversible or expensive decisions.
- `TRACEABILITY.md` maps requirements to contracts, tests and release gates.
- The archived Master Plan is **not normative** after migration.

## Change rule

Any PR that changes a public rendering contract must update at least one of:

- subsystem specification;
- ADR;
- traceability row;
- release gate / test requirement.

A code change that silently changes architecture without a corresponding documentation change is incomplete.

## Validation

Run:

```bash
python3 tools/check_docs.py
```

The checker validates ADR IDs/statuses, Accepted ADR completeness, Traceability coverage, local documentation paths, archive safety, required governance files and legacy-term regressions.

Documentation package provenance is recorded in `DOCS_METADATA.toml`; file integrity is recorded in `DOCUMENT_MANIFEST.json`.
