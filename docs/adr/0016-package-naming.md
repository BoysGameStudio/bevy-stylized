# ADR-0016: Package Naming

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-11

## Context

The original working name `bevy-npr` cannot be treated as a clean publishable package identity because an existing public `bevy_npr` crate already occupies the hyphen/underscore-equivalent namespace.

The framework also needs a name broad enough to cover Cel/Anime rendering today and future Ink/Comic/Watercolor or other non-photorealistic styles without binding the renderer to one game/reference family.

## Options

1. **Bevy NPR** — technically accurate but registry-conflicted and less approachable as a product name.
2. **Bevy Toon / Bevy Cel** — clear but too narrow for the planned profile ecosystem.
3. **Bevy Gacha / Bevy Anime** — binds the framework to one content/style category.
4. **Bevy Stylized** — broad, readable, and aligned with the long-term framework scope.

## Decision

Adopt **Bevy Stylized** as the project/framework brand.

Canonical naming:

```text
GitHub repository / project directory:
bevy-stylized

Cargo package / crate directories:
bevy-stylized
bevy-stylized-core
bevy-stylized-render
bevy-stylized-gltf
bevy-stylized-profile-genshin
bevy-stylized-tools

CLI binary:
bevy-stylized
```

Rust identifiers follow Cargo's normal hyphen-to-underscore mapping:

```rust
bevy_stylized
bevy_stylized_core
bevy_stylized_render
bevy_stylized_gltf
bevy_stylized_profile_genshin
bevy_stylized_tools
```

Public framework API types use the **`Stylized*`** prefix where a project-level namespace is useful, for example:

```rust
StylizedPlugin
StylizedMaterialDescriptor
StylizedSurfaceClass
StylizedExecutionTier
StylizedCapabilities
```

`NPR` remains valid as a technical/domain term for algorithms or literature-facing concepts such as `NprLightingModel`; the brand decision does not require renaming every NPR algorithm term.

The application/schema namespace is:

```text
bevy_stylized.*
```

Schema versioning remains independent of Cargo package versioning.

## Evidence

- An existing public `bevy_npr` crate makes `bevy-npr` an unsuitable package identity because Cargo/crates.io treats hyphen/underscore-equivalent names as colliding.
- A 2026-09-11 public web-index check found no exact `bevy-stylized` / `bevy_stylized` crate or docs.rs result. This is **not a namespace reservation**; availability must be rechecked immediately before the first crates.io publish.
- `Stylized` matches the architecture scope better than `Toon`, `Cel`, `Anime`, or a game/reference-family name.

## Consequences

- Repository and workspace documentation must use kebab-case package/directory names.
- Rust examples must use snake_case crate identifiers.
- The facade package is `bevy-stylized`; internal packages use the same prefix.
- CLI examples use `bevy-stylized`.
- Existing `bevy_npr` metadata/schema working names migrate to `bevy_stylized`.
- Before first crates.io publication, the release process must perform a final registry/name/trademark preflight; if the package namespace becomes unavailable before publication, this ADR must be revisited.

## Revisit trigger

Revisit only if:

- `bevy-stylized` becomes unavailable in the target registry before first publication;
- a material trademark/legal naming issue is discovered;
- the project scope changes so substantially that `Stylized` becomes misleading.
