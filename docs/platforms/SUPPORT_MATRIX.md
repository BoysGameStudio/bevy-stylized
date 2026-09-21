# Platform Support Matrix

> **Status:** Evidence ledger. `Supported` is never inferred from compile success.

## Evidence states

- ✅ **Validated** — evidence exists for this cell.
- 🧪 **Experimental** — partial implementation/evidence exists.
- ⚪ **Not tested** — no qualification evidence yet.
- 🚧 **Blocked** — known blocker or unresolved ADR prevents qualification.
- ❌ **Unsupported** — intentionally unsupported for the stated release.
- N/A — not applicable.

| Target | Architecture | Compile | Shader | Pipeline | Runtime | Device | Performance | Public status |
|---|---|---|---|---|---|---|---|---|
| Windows / D3D12 | ✅ Compatible | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | Not qualified |
| Linux / Vulkan | ✅ Compatible | ✅ cargo test --workspace（本机） | ✅ naga 全量校验 + 真实管线创建（golden/budget 测试） | ✅ 全 class pipeline 创建（golden/budget 测试） | ✅ golden 捕获全绿（本机） | ✅ RTX 5090（本机 baseline，provenance 记录于各 `*.golden.json`） | 🧪 本地 CPU sweep 与单视图 GPU 时间戳；绝对预算未定义 | Experimental（本机证据，single-device） |
| Linux / GLES | ✅ Compatible | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | Not qualified |
| macOS / Metal | ✅ Compatible | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | Not qualified |
| iOS / Metal | ✅ Compatible | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | Not qualified |
| Android / Vulkan | ✅ Compatible | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | Not qualified |
| Android / GLES | ✅ Compatible | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | Not qualified |
| Web / WebGPU | ✅ Compatible | ✅ standalone wasm32 | ✅ real browser compilation | ✅ Cel/Character/Hero + hull | ✅ static asset capture | ✅ Linux/Chrome 153, RTX 5090 | ⚪ Not measured | Experimental, scoped below |
| Web / WebGL2 | Outside current commitment | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | No support promise |
| Consoles | Port-ready architecture only | N/A | N/A | N/A | N/A | N/A | N/A | Port-ready |

## Public status definitions

**Supported** requires compile + shader + backend pipeline + representative real runtime render + showcase smoke + release regression evidence.

**Experimental** means architecture/compile/pipeline work exists but runtime/device/performance qualification is incomplete.

**Port-ready** only means there is no known architectural lock-in; it is not a shipping-support claim.

## Evidence records

When a cell becomes `Validated`, link or record an local evidence ID (device record, Golden run, benchmark report) in the release qualification record.

## Compatibility statement（provisional pending ADR-0027）

- **Bevy**: `=0.19.1`（pinned；V1 开发期锁定 0.19.x，升级走 ADR-0027 正式流程）；
- **Rust toolchain**: stable 1.98.x（`rust-toolchain.toml` 锁定）；
- **实测设备**：Linux x86_64 / NVIDIA RTX 5090（driver 615.71.09）；Vulkan 原生回归及以下 BrowserWebGpu 记录均来自该机；
- 其余目标一律按上表的 Not qualified / Experimental 处理。

## WebGPU local record, 2026-09-21

Run `hardening-webgpu`, report `target/stylized/hardening-webgpu/report.json`:
Chrome 153.0.8010.52 on Wayland, Bevy 0.19.1, pinned Rust 1.98.0;
actual backend `BrowserWebGpu`, browser adapter NVIDIA/Blackwell. A standalone
registry consumer compiled outside the parent workbench; its lock SHA-256 is
`3f22dc2109a3c4b20c8edab59eb64908ca467e09dca29e643ce8c7be3a627c5c`.
The 512² fixture adopted all 10 compiled static-character primitives, created
8 hull outlines and rendered procedural Cel/Character/Hero samples. Browser
and native Vulkan RGBA readbacks were identical (maximum channel delta 0), with
no reported shader compilation or uncaptured GPU errors. The required flags and
reproducible local command are in [Contributing](../../CONTRIBUTING.md#browser-webgpu).

The successful adapter required a headed browser and explicit WebGPU/Vulkan flags;
headless discovery returned no adapter. This qualifies neither default browser
configuration nor another device/OS. WebGPU shadow/prepass, instance tables,
deformation, performance and mobile browsers remain untested. WebGL2 is outside
the owner's current support promise. Native screenshot parity does not establish
upstream Blender parity; that uses the separate strict character lane.
