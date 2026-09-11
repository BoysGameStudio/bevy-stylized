# 从 Tenacity 迁移的实施路线

> **Status:** Normative architecture specification
> **Authority:** This document is authoritative for its domain; the archived Master Plan is historical only.


迁移必须双轨进行：

```text
Legacy Reference Path
+
New Production Path
```

不能：

```text
先删旧 Shader
→ 凭肉眼重新实现
```

否则失去 Upstream Reference-Parity Oracle。

---

## Phase 0：Reference Freeze

1. 固定当前 Gacha/Blender Reference Shader；
2. 创建 `Reference Toolchain Lock`，锁定 Blender 精确版本、Gacha Setup release/tag/commit、release asset digest；
3. 固定 Blender glTF Export Contract；
4. 建立 Gacha Setup Upstream Behavior Contract；
5. 建立 Blender Reference Export Bridge，并让它输出 Node-Graph Signature 与 Export Manifest；
6. 修复影响 Reference 正确性的明确 bug；
7. 固定 Reference Scene、Camera、Lighting、Resolution；
8. 建立带 Provenance Metadata 的 Golden；
9. 建立 Reference Upgrade Policy；
10. 将 Reference 与 Shipping Production 解耦。

---

## Phase 1：Correctness

优先完成：

1. InternalEdge ABI；
2. Character Cull Inherit；
3. Real Pipeline Creation Test；
4. WGSL/Naga Validation；
5. Outline Lifecycle Contract；
6. Camera/Post Ownership。

---

## Phase 2：Core Extraction

建立：

```text
bevy-stylized-core
```

抽离：

- Ramp；
- Face settings；
- Lighting model；
- Material descriptor；
- Instance descriptor；
- Capability；
- Quality；
- Profile contract。

这一阶段不追求 GPU 优化。

---

## Phase 3：Cel Surface + Portable Execution Baseline

实现：

```text
Cel @ Portable
```

先满足：

- minimum resource budget；
- fixed binding；
- core ramp/control；
- registered `_NPR_*` glTF attributes + typed extras/sidecar import；
- hull outline；
- basic physical/authored lighting；
- Pipeline/Golden。

这是整个新 Renderer 的地基。

---

## Phase 4：Material / Instance / View

完成：

```text
Material
Instance
View
```

分离。

迁移：

- Face Runtime State；
- Key Light；
- Global Palette；
- Day/Night；
- Outline Instance Control。

消除核心路径的 per-frame Material Asset mutation。

---

## Phase 5：Character

加入：

- Face；
- Hair；
- Rim；
- Emission；
- Enhanced Specular。

必须定义 Character @ Portable fallback/approximation。

---

## Phase 6：Hero Surface + Genshin Reference Family

加入：

- Gacha ILM；
- Authored Ramp；
- Packed Normal；
- MatCap；
- Spec Ramp；
- Face Correction；
- Gacha Palette。

同时建立：

```text
Upstream Golden → Engine Reference → Hero Production
```

Golden 对比。

---

## Phase 7：GPU Resource Architecture

执行：

- UBO packing；
- sampler dedup；
- texture budget；
- vertex control packing；
- bindless prototype；
- fixed/bindless comparison。

在这里通过 ADR 决定最终 Runtime Material Backend。

---

## Phase 8：Showcase / Tools / Docs

完成：

- Showcase labs；
- validate/inspect CLI；
- Integration docs；
- Custom Profile example；
- Migration docs。

---

## Phase 9：V1 Freeze

停止新增大 Feature。

只处理：

- correctness；
- portability；
- API cleanup；
- performance regression；
- documentation。

---

## 迁移状态记录

### 2026-09-11：Legacy Reference Path 导入

- 从 Tenacity(local:`/home/x/Projects/Tenacity`,commit `b5b26ac`)拷贝 `bevy-cel` 与 `bevy-cel-metadata` 到 `crates/legacy/`,package 名保持 `bevy_cel` / `bevy_gltf_metadata`,`embedded://` shader 路径不变。
- 两个 crate 冻结为 Reference Oracle;其 public API 不是 Bevy Stylized 的稳定契约。Production Path(Phase 2 起）据此做 Core Extraction 与对照验证。
- 剥离了 `bevy_gltf_metadata` 的可选 `kawaii` feature（依赖未迁移的 `bevy_kawaii_physics`,bevy_cel 未使用）。
- 未导入 git 历史；本节即 provenance 记录。许可证沿用原 MIT 声明，仓库级许可证仍由 ADR-0017 决定。

---
