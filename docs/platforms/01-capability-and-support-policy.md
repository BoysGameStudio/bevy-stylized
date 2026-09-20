# 跨平台 Capability 与支持策略

> **Status:** Normative architecture specification


## Portable Baseline 必须分开定义

Portable 不能同时模糊表示 WebGPU、GLES3.1、WebGL2/GLES3.0。

正式 baseline：

```text
Baseline A: WebGPU Core / modern wgpu limits
Baseline B: GLES 3.1 / wgpu downlevel_defaults
Baseline C: GLES 3.0 + WebGL2 / wgpu downlevel_webgl2_defaults
```

wgpu 明确保证：`downlevel_defaults()` 对 GLES 3.1 兼容；`downlevel_webgl2_defaults()` 对 GLES 3.0/WebGL2 兼容，后者 storage buffer/storage texture 保证值为 0，per-stage UBO 保证值为 11。

V1 必须在 ADR 中明确 Web Policy：

```text
Option 1: WebGPU-only web support
Option 2: WebGPU + WebGL2 fallback
```

在该 ADR 通过前，README 不能笼统声明“Web supported”。Bevy 0.19 官方仍把 WebGPU 标记为 experimental，因此即使 NPR 自身满足 WebGPU limits，也必须把 Bevy upstream status 纳入 Support Claim。

---

## Capability Resolver 必须分成 Renderer-Init 与 Runtime 两阶段

wgpu/Bevy 中必须区分：

```text
Adapter supports feature/limit
!=
Device actually requested/enabled feature/limit
```

Bevy 0.19 的 `WgpuSettings` 在 RenderDevice 创建前控制 backend、priority、features、limits、constrained_limits 等；RenderDevice 创建完成后，应使用 `RenderDevice::features()` / `RenderDevice::limits()` 与 `RenderAdapterInfo` 获取**实际启用**的能力。

因此架构必须分两阶段：

### Stage A：RenderDevice Creation Policy（初始化前）

```text
Application policy
+
Bevy RenderPlugin/WgpuSettings
+
Backend/Web policy
→ requested Device features/limits
```

V1 默认 NPR 插件必须是**被动兼容**的：

- 不要求用户为了 Cel/Portable 修改 WgpuSettings；
- 不在 RenderDevice 创建以后尝试“补申请” feature；
- High/Bindless 等路径只在实际 RenderDevice 已启用所需能力时选择。

如果某个未来 High feature 确实需要在 device creation 阶段显式 request，则 facade 必须提供**初始化前**的配置 helper/文档，并说明它必须与 `DefaultPlugins.set(RenderPlugin { ... })` 一起使用。普通 `.add_plugins(StylizedPlugin)` 不能 retroactively 修改已创建 Device。

### Stage B：Runtime Capability Resolution（初始化后）

权威输入：

```text
RenderDevice::features()
RenderDevice::limits()
RenderAdapterInfo
Compiled Capabilities
Application Quality Policy
Surface Requirements
```

输出：

```text
StylizedCapabilities
      ↓
Resolved SurfaceClass × ExecutionTier Variant
```

禁止：

```text
adapter 理论支持
→ 假定 device 一定启用了该 feature
```

也禁止把 pipeline creation failure 当 feature detection。

### Test Profiles

CI/QA 应显式覆盖不同 Device Policy，例如：

```text
WebGPU-like limits
GLES3.1 downlevel limits
WebGL2/GLES3.0 limits
Functionality/high-end limits
```

并记录 Bevy/WgpuSettings priority 与实际 RenderDevice limits。

---

## Quality Policy

默认：

```rust
StylizedQualityPolicy::Auto
```

同时允许：

```rust
StylizedQualityPolicy::Force(StylizedExecutionTier::Portable)
```

用途：

- QA；
- Benchmark；
- Compatibility；
- Low-spec simulation。

Reference 使用显式 Debug/QA 模式，不应被 Auto 自动选择。

---

## 确定性降级策略

每个高级 Feature 必须声明 fallback。

| Feature | High Execution / Hero Surface（按适用条件） | Portable Execution Fallback |
|---|---|---|
| Internal Edge | Depth/Normal Edge | Hull only |
| Packed Source Normal | Full reconstruction | Standard/vertex normal approximation |
| Metallic Spec Ramp | Full | Simplified specular |
| Face Correction | Full | Basic Face SDF |
| HDR Emission | HDR | LDR-safe equivalent |
| Chromatic Post | Optional | Disabled |
| Bindless | When supported/beneficial | Fixed binding |

Capability Resolver 只能选择显式注册的 fallback。

禁止：

```text
资源不够
→ 随机关掉一张 Texture
```

---

## 平台支持状态必须分层

公开 README 不应只写“Supported”。

应区分：

```text
Architecture-Compatible
Compile-Validated
Shader-Validated
Pipeline-Validated
Render-Validated
Device-Validated
Performance-Qualified
```

正式平台证据矩阵只维护一份：

[`SUPPORT_MATRIX.md`](SUPPORT_MATRIX.md)

本规范定义状态语义；`SUPPORT_MATRIX.md` 记录每个平台当前证据状态，避免出现第二份手工同步的状态表。

---

## 主机平台

在没有：

- 平台 SDK；
- 厂商 backend；
- 真实主机；
- 性能验证；

之前只能声明：

```text
Console-port-ready architecture
```

不能声明：

```text
Console supported
```

---

## Supported / Experimental / Port-ready 的严格定义

公开仓库只允许三种对外状态：

### Supported

必须同时满足：

```text
Compile
+
Shader Validation
+
Pipeline Validation
+
Representative Real Runtime Render
+
Showcase Smoke Test
+
Release Regression Suite
```

对于标记为 `Supported` 的平台，不能用模拟器、交叉编译或 Pipeline 创建代替真实平台运行证据。

### Experimental

至少：

```text
Compile
+
Shader/Pipeline Validation
```

可以缺少完整 Device/Performance Coverage，但 README 必须明确限制。

### Port-ready

仅表示：

```text
Core Architecture 没有已知平台锁定
```

不代表已经存在 backend、包体、输入层或真实硬件验证。

主机在没有真实 SDK/Backend/Device 前只能是 `Port-ready`。

---

## V1 平台发布门槛

如果 V1 对某个平台写 `Supported`，Release Checklist 必须保存至少一份可追踪的真实运行记录。

最低目标矩阵：

```text
Windows / D3D12
Linux / Vulkan
macOS / Metal
iOS / Metal
Android / Vulkan
Android / GLES（若项目声明支持 GLES）
Web / WebGPU
Web / WebGL2（仅当 Web Policy 声明支持）
```

WebGPU 还应至少覆盖发布时选定的代表性浏览器/OS 组合，并记录浏览器版本。

不要求所有这些验证都运行在 GitHub-hosted CI，但必须：

```text
Automated CI
+
Release Device Matrix
```

共同组成发布证据。

---
