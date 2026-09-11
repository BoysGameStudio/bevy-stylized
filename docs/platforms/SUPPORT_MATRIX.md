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
| Linux / Vulkan | ✅ Compatible | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | ⚪ Not tested | Not qualified |
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
