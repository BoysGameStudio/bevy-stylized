# 工具链、版本、发布与公共仓库规则

> **Status:** Normative architecture specification
> **Authority:** This document is authoritative for its domain; the archived Master Plan is historical only.


## Package Naming 与 License/Provenance Gate

### Package Naming

ADR-0016 已接受：

```text
Project / GitHub repository / workspace directory:
bevy-stylized

Facade Cargo package:
bevy-stylized

Rust crate identifier:
bevy_stylized
```

子包统一：

```text
bevy-stylized-core
bevy-stylized-render
bevy-stylized-gltf
bevy-stylized-profile-genshin
bevy-stylized-tools
```

对应 Rust identifier：

```text
bevy_stylized_core
bevy_stylized_render
bevy_stylized_gltf
bevy_stylized_profile_genshin
bevy_stylized_tools
```

命名规范：

```text
filesystem / GitHub / Cargo package → kebab-case
Rust crate identifier               → snake_case
public framework types              → Stylized*
technical NPR algorithm terms       → Npr* allowed when semantically appropriate
```

原 `bevy_npr` 名称存在 registry 冲突，因此不再作为项目/package 名。首次 crates.io 发布前仍必须重新检查 `bevy-stylized` 的实时可用性；命名选择不等于 namespace reservation。

### License

当前 Reference 上游 Gacha Setup 及其内置 Genshin shader lineage 声明 GPL-3.0。文档不能预先宣称新仓库一定 MIT/Apache。

必须完成 `License & Provenance ADR`：

```text
Independent implementation
Reference-only behavior
Copied/transformed code
Bundled shader artifact
Bundled asset
```

逐类记录来源与许可。公开发布前应进行专门的许可证/法律审查。

---

## CLI

V1：

```bash
bevy-stylized validate character.glb
bevy-stylized inspect character.glb
```

并必须提供可脚本化的：

```text
Blender Reference Export Bridge
```

用于固定 Blender/Gacha Setup Reference → glTF/metadata/manifest 的转换。

Validate：

- UV；
- Tangent；
- Custom Attributes；
- Face Rig；
- Texture Format；
- Profile Requirements；
- Portable Compatibility。

Inspect：

- Required Class；
- Fallback；
- Texture Count；
- Sampler Count；
- Vertex Attributes；
- Profile；
- Resource Budget。

V2：

```text
compile
optimize
golden
benchmark
```

---

## Cargo Feature：只表达 Compile Capability，不表达 Runtime Tier

Cargo features 是 additive union；不能把它当成 runtime quality switch。

推荐 V1 Feature 主要用于可选子系统：

```toml
[features]
default = ["gltf"]

gltf = ["dep:bevy-stylized-gltf"]
profile-genshin = ["dep:bevy-stylized-profile-genshin"]
reference = ["profile-genshin"]
devtools = []
bindless-runtime = []        # compile capability only
webgl2-compat = []           # only if Web Policy accepts this target
```

如果为了最终二进制体积需要裁掉生产 SurfaceClass shader，可以额外使用 additive：

```text
runtime-character
runtime-hero
```

但运行时最终 Variant 必须始终由：

```text
Compiled Capabilities
∩ Device Capabilities
∩ Surface Requirements
∩ Runtime Quality Policy
=
Resolved Variant
```

决定。

---

## Feature Matrix CI

必须测试：

- default features；
- no-default-features；
- minimal/default；
- runtime-character on/off；
- runtime-hero on/off；
- gltf on/off；
- profile-genshin；
- reference；
- bindless compile path。

可以使用 `cargo hack` 或等价工具自动覆盖 feature powerset 的合理子集。

---

## Bevy 版本策略

Bevy minor 之间可能有明显 Renderer/API 变化。

因此：

```text
`bevy-stylized` release line
→ 明确支持一个 Bevy minor 或经过验证的兼容范围
```

每次升级 Bevy 必须执行：

- API Migration；
- Shader Suite；
- Pipeline Suite；
- Golden；
- Performance；
- Examples；
- Showcase。

不承诺“同一个 release 自动支持所有 Bevy minor”。

---

## Rust/MSRV 策略

公开仓库应明确：

```text
MSRV / Rust toolchain policy
```

可以选择：

- 跟随 Bevy MSRV；
- 或锁定 workspace toolchain。

CI 至少测试官方声明的最低 Rust 版本和当前 stable。

---

## SemVer

`0.x`：

- 可 breaking；
- 必须 CHANGELOG；
- 必须 Migration Note。

`1.0` 后稳定：

- facade API；
- Authoring Schema；
- Profile Contract；
- 公开 Serialization Contract。

Renderer internal 类型尽量保持私有。

---

## 资产与版权

公共仓库不能直接分发未授权第三方游戏资产。

允许：

```text
公开 synthetic fixtures
+
自有 Showcase assets
+
明确许可资产
```

如果某些 Upstream Reference-Parity 测试必须依赖私有资产：

```text
Public Synthetic Suite
+
Optional Private/Local Reference Suite
```

公共 CI 不能依赖无法合法分发的文件。

必须维护：

```text
THIRD_PARTY_NOTICES.md
```

---

## Reference 的 Shipping 边界

Reference 默认：

```text
off
```

普通发行只包含 Production Runtime。

QA：

```text
--features reference
```

避免：

- 大型 reference shader；
- debug path；
- private test asset hooks；

进入 Shipping Build。

---

## Observability

Renderer 必须能解释自己的决策。

至少显示：

- Resolved Tier；
- Material Class；
- Lighting Backend；
- Bindless；
- Texture/Sampler/UBO Count；
- Draw Calls；
- Visible NPR Instances；
- Outline Mode；
- Internal Edge；
- HDR/Post State。

V2 增加：

- GPU timings；
- LOD distribution；
- Dynamic Quality reason；
- streaming residency。

---
