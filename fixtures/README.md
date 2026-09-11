# Fixtures

## `golden-local/` (gitignored, local only)

Golden captures produced by `crates/dev/odette-reference-lab` use the Odette
character from the sibling Tenacity project. Odette is third-party
HoYoverse-derived art, so every capture under `fixtures/golden-local/` is
local-only: the PNGs and provenance sidecars are referenced by path and never
committed (see `docs/adr/0017-license-and-provenance.md` §Partial decision).

Regenerate or accept a new baseline locally with:

```sh
GOLDEN_REBASELINE=1 cargo run -p odette-reference-lab
```

Repo-committed golden baselines (first-party synthetic fixtures) belong under
`fixtures/golden/` instead; none exist yet.
