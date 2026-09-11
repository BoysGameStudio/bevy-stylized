# Glossary

| Term | Definition |
|---|---|
| **Surface Class** | Semantic material complexity: `Cel`, `Character`, or `Hero`. It describes what the surface needs to mean, not how expensive it may run. |
| **Execution Tier** | Runtime quality/performance policy: `Portable`, `Standard`, or `High`. |
| **Validation Mode** | Reference/QA execution used to validate production output; not a runtime quality tier. |
| **Resolved Runtime Variant** | Concrete pipeline/material implementation selected from Surface Class × Execution Tier × actual device capabilities × profile fallback rules. |
| **Profile** | Art-style-specific semantic mapping layered over generic NPR primitives. |
| **Reference Family** | Versioned lineage of upstream visual behavior, e.g. the V1 Genshin/Gacha Setup family. |
| **Upstream Golden** | Image rendered directly from the locked Blender/reference toolchain before runtime export. |
| **Engine Reference Renderer** | High-fidelity engine implementation used to validate authoring translation and production optimization. |
| **Production Renderer** | Shipping runtime implementation optimized for supported devices. |
| **Material Data** | Static/shared surface state. |
| **Instance Data** | Per-render-instance dynamic NPR state such as face threshold/side or instance overrides. |
| **View Data** | Camera/frame/global lighting and quality state. |
| **Portable Baseline** | Explicit device/backend limit profile; WebGPU Core, GLES 3.1 and WebGL2/GLES 3.0 must not be conflated. |
| **Whole-Pipeline Budget** | Per shader-stage resources consumed by all visible bind groups: view + mesh + instance + material + profile + safety margin. |
| **Fallback Contract** | Deterministic downgrade from a feature/variant to a supported lower-cost semantic equivalent. |
| **Reference-Parity** | Parity with the locked upstream Blender/reference implementation, not an unproven claim of 100% parity with a commercial game's renderer. |
| **ADR** | Architecture Decision Record: context, options, decision, evidence, consequences and revisit trigger. |
| **Supported** | Platform status backed by compile, shader, pipeline, real runtime, showcase smoke and release regression evidence. |
| **Experimental** | Compiles and validates but lacks full runtime/device/performance qualification. |
| **Port-ready** | No known architecture lock-in; not a support claim. |
