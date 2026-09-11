# 独立 GitHub 仓库与 Crate 架构

> **Status:** Normative architecture specification
> **Authority:** This document is authoritative for its domain; the archived Master Plan is historical only.


## 仓库、目录与 Cargo Package 命名

项目正式品牌与仓库名：

```text
Bevy Stylized
bevy-stylized
```

命名规则：

| Surface | Convention | Example |
|---|---|---|
| GitHub repository | kebab-case | `bevy-stylized` |
| Project/workspace directory | kebab-case | `bevy-stylized/` |
| Cargo package / crate directory | kebab-case | `bevy-stylized-core/` |
| Rust crate identifier | snake_case | `bevy_stylized_core` |
| Public framework type | PascalCase `Stylized*` | `StylizedPlugin` |
| Technical NPR algorithm term | keep `Npr*` when semantically useful | `NprLightingModel` |
| Authoring/schema namespace | snake_case brand namespace | `bevy_stylized.material/v1` |

Cargo package availability must still be rechecked immediately before the first crates.io publish; the naming decision itself is closed by ADR-0016.

仓库许可证仍不能在尚未完成 GPL/Reference provenance 审查前直接假定为 MIT/Apache 双许可。Gacha Setup 和当前 Genshin Reference artifact lineage 均声明 GPL-3.0；最终仓库许可证由 ADR-0017 决定。

V1 仓库结构：

```text
bevy-stylized/
│
├── Cargo.toml
├── Cargo.lock
├── README.md
├── ARCHITECTURE.md
├── CHANGELOG.md
├── CONTRIBUTING.md
├── LICENSE.md                 # pending ADR-0017/review
├── THIRD_PARTY_NOTICES.md
│
├── crates/
│   ├── bevy-stylized/                    # facade
│   ├── bevy-stylized-core/
│   ├── bevy-stylized-render/
│   ├── bevy-stylized-gltf/
│   ├── bevy-stylized-profile-genshin/    # V1 concrete reference family
│   └── bevy-stylized-tools/
│
├── shaders/
│   ├── common/
│   ├── cel/
│   ├── character/
│   ├── hero/
│   ├── outline/
│   ├── post/
│   └── reference/
│
├── examples/
├── showcase/
├── fixtures/
├── tests/
├── benches/
├── tools/blender-reference-export/
├── docs/
│   ├── adr/
│   ├── architecture/
│   ├── authoring/
│   ├── profiles/
│   ├── platforms/
│   ├── reference/
│   ├── performance/
│   ├── quality/
│   └── migration/
└── .github/workflows/
```

早期命名 ADR 已关闭：

```text
ADR-0016 Package Naming → Accepted: bevy-stylized
ADR-0017 License & Provenance → still open
```

---

## V1 不应过度拆 Crate

V1 控制到：

```text
bevy-stylized
bevy-stylized-core
bevy-stylized-render
bevy-stylized-gltf
bevy-stylized-profile-genshin
bevy-stylized-tools
```

Outline / Post / Runtime Asset 先作为 `bevy-stylized-render` module。

只有出现清晰独立 API 与独立版本价值时，再拆：

```text
bevy-stylized-outline
bevy-stylized-post
bevy-stylized-assets
```

---

## 依赖方向

必须单向：

```text
Showcase / Tools / Examples
            │
            ▼
Profiles / glTF
            │
            ▼
Renderer
            │
            ▼
Core
```

禁止：

```text
Core → Gacha
Core → Showcase
Renderer → Tenacity
```

推荐：

```text
bevy-stylized
├── bevy-stylized-core
├── bevy-stylized-render
├── optional bevy-stylized-gltf
└── optional bevy-stylized-profile-genshin

bevy-stylized-render
└── bevy-stylized-core

bevy-stylized-gltf
└── bevy-stylized-core

bevy-stylized-profile-genshin
├── bevy-stylized-core
└── 尽量不接触 render internals

bevy-stylized-tools
├── bevy-stylized-core
└── bevy-stylized-gltf
```

---

## Facade

对外稳定 facade package：

```toml
[dependencies]
bevy-stylized = "0.1"
```

Rust import：

```rust
use bevy_stylized::prelude::*;
```

目录/Cargo package 使用中横线；Rust crate identifier 使用 Cargo 自动映射后的下划线。

Facade 必须同时提供：

```text
StylizedPlugin                  # programmatic/no-glTF path
Stylized-aware `GltfPlugin` configuration helper # _NPR_* registration before loader initialization
```

不能承诺“DefaultPlugins 加完以后再加 StylizedPlugin 就能 retroactively 注册 glTF custom attributes”。

Facade 只负责：

- Re-export；
- Plugin Group；
- compile capability routing；
- 稳定公共 API。

Renderer internals 尽量 `pub(crate)`，不要把 Bevy Render World internals 变成 1.0 稳定 API。

---

## Core

`bevy-stylized-core` 只包含：

- Domain Model；
- `StylizedMaterialDescriptor`；
- `StylizedInstanceDescriptor`；
- `StylizedViewDescriptor`；
- `StylizedSurfaceClass` / `StylizedExecutionTier` / `StylizedValidationMode`；
- Capability Model；
- Quality Policy；
- Profile Contract；
- Schema-neutral serialization types。

尽量避免依赖完整 `bevy` crate。

能使用：

```text
bevy_math
bevy_color
bevy_reflect（需要时）
```

就不要为了几个类型依赖完整 Renderer。

Core 不应出现：

```text
RenderPipelineDescriptor
BindGroup
TextureView
CommandEncoder
RenderPhase
```

---
