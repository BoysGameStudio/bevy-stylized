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
| Linux / Vulkan | ✅ Compatible | ✅ cargo test --workspace（本机） | ✅ naga 全量校验 + 真实管线创建（golden/budget 测试） | ✅ 全 class pipeline 创建（golden/budget 测试） | ✅ golden 捕获全绿（本机） | ✅ RTX 5090（本机 baseline，provenance 记录于各 `*.golden.json`） | 🧪 spike 测量存在（adr-0001/0004），无 CI 基线 | Experimental（本机证据，single-device） |
| Linux / GLES | ✅ Compatible | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | Not qualified |
| macOS / Metal | ✅ Compatible | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | Not qualified |
| iOS / Metal | ✅ Compatible | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | Not qualified |
| Android / Vulkan | ✅ Compatible | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | Not qualified |
| Android / GLES | ✅ Compatible | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | Not qualified |
| Web / WebGPU | ✅ Compatible | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | Experimental until qualified |
| Web / WebGL2 | 🚧 Candidate | 🚧 ADR-0021 | 🚧 ADR-0021 | 🚧 ADR-0021 | 🚧 ADR-0021 | 🚧 ADR-0021 | 🚧 ADR-0021 | Policy unresolved |
| Consoles | Port-ready architecture only | N/A | N/A | N/A | N/A | N/A | N/A | Port-ready |

## Public status definitions

**Supported** requires compile + shader + backend pipeline + representative real runtime render + showcase smoke + release regression evidence.

**Experimental** means architecture/compile/pipeline work exists but runtime/device/performance qualification is incomplete.

**Port-ready** only means there is no known architectural lock-in; it is not a shipping-support claim.

## Evidence records

When a cell becomes `Validated`, link or record an evidence ID (CI run, device record, Golden run, benchmark report) in the release qualification record.

## Compatibility statement（provisional pending ADR-0027）

- **Bevy**: `=0.19.1`（pinned；V1 开发期锁定 0.19.x，升级走 ADR-0027 正式流程）；
- **Rust toolchain**: stable 1.98.x（`rust-toolchain.toml` 锁定）；
- **有实测证据的唯一平台**：Linux x86_64 / Vulkan / NVIDIA RTX 5090（driver 615.71.09）——golden 捕获、naga 校验、benchmark 基线均产自该机；
- 其余目标一律按上表的 Not qualified / Experimental 处理。
