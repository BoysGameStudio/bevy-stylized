# Reference Documentation

Normative documents:

1. `01-reference-authoring-workflow.md`
2. `02-reference-toolchain-lock.md`
3. `03-blender-export-contract.md`
4. `04-upstream-contract-upgrade-and-golden.md`
5. `source-register.md`
6. `05-license-and-provenance-boundary.md`

The validation chain is:

```text
Upstream Blender Golden
        ↓
Engine Reference Renderer
        ↓
Production Renderer
```

Do not bypass the upstream Golden by validating Production only against engine-generated output.
