# Current implementation and qualification sequence

This page routes current work; it does not preserve completed migration stages or
turn future roadmap options into implementation authorization.

1. Confirm the affected contract, current capability/profile scope and source lock.
2. Update the owning product module and current consumers together; keep engine,
   authoring and application responsibilities separate.
3. Run the applicable local CPU/schema/independent-consumer checks, then the
   required GPU/asset/current-character case when rendering or authoring changes.
4. Report actual results and unresolved requirements through the current
   [release checklist](../release/V1_RELEASE_CHECKLIST.md) and [ADR index](../adr/ADR_INDEX.md).

Do not create remote CI or retain an old renderer/addon version to satisfy a
historical milestone. Current license provenance remains in
[third-party notices](../../THIRD_PARTY_NOTICES.md). New features and target
platforms require their own current contract and measured qualification.
