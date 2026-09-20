# Renderer ≠ Art Style

> **Status:** Normative architecture specification


Core 提供：

- Ramp；
- Face；
- Hair；
- Rim；
- Specular；
- MatCap；
- Outline；
- Lighting；
- Post primitives。

Profile 决定如何组合。

---

## Profile Contract

Profile 负责 Art-style policy，不负责 GPU backend 选择。

建议逻辑接口（仅表达语义，不承诺最终 Rust ABI）：

```rust
pub trait StylizedProfile {
    fn surface_class(&self) -> StylizedSurfaceClass;

    fn prepare_material(
        &self,
        source: &StylizedSourceMaterial,
        output: &mut StylizedMaterialDescriptor,
    );

    fn prepare_instance(
        &self,
        context: &StylizedInstanceContext,
        output: &mut StylizedInstanceData,
    );

    fn fallback_contract(
        &self,
        tier: StylizedExecutionTier,
    ) -> StylizedProfileFallbackContract;
}
```

关键边界：

```text
Profile
→ 声明 Surface semantics + fallback contract

Capability Resolver
→ 根据实际 RenderDevice + Execution Policy 选择实现
```

Profile 不应直接查询/绑定 wgpu backend internals。

V1 需要稳定的是**Profile 语义与 Versioned Schema Contract**；具体 Rust trait signature 在 1.0 前可以调整。V3 所谓“稳定 Profile”默认指 Stable API + Versioned Schema，而不是未经设计的二进制 Rust ABI。

---

## Reference Family：V1 锁定 Genshin，而不是抽象“Gacha”

Gacha Setup 是一个聚合工具，当前同时覆盖 Genshin、HSR、ZZZ、NTE、WuWa 等，而这些游戏引用的是不同 shader lineage。它们不能在 V1 被假定为共享同一套 shading contract。

V1 只定义一个具体、可追溯的：

```text
Genshin Reference Family
```

其上游至少包括：

```text
Gacha Setup locked release
+
Festivity/Blender-miHoYo-Shaders lineage
+
locked Genshin main .blend artifact
+
locked outline .blend artifact
+
locked post-processing .blend artifact
```

V1 profile crate/模块使用工作名：

```text
profile-genshin
```

未来如支持 HSR/ZZZ/WuWa，应分别建立：

```text
HsrReferenceFamily
ZzzReferenceFamily
WuwaReferenceFamily
```

它们可以在更高层被某个 `gacha` collection/facade 聚合，但不能把所有 shader family 的特殊逻辑塞进一个 Hero shader/profile。

文档中的“Upstream Reference-Parity”统一解释为：

> **Upstream Reference-Parity：与锁定的 Blender/Gacha Setup/具体 shader artifact Reference 保持定义范围内的一致。**

它不等价于“与原始商业游戏 renderer 100% 一致”。若未来需要 Game Capture Parity，必须建立独立验证轨道。

---
