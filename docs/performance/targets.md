# Performance Targets

> **Status:** Normative format; concrete budgets are release/device data, not architecture constants.

Architecture documents do not hard-code one universal millisecond target. Performance qualification is relative to a named device/profile and a fixed visual-quality contract.

## Required target record

Every performance target must record:

```text
Target ID
Release / commit
Device / GPU / driver / OS
Backend
Resolution / dynamic-resolution policy
Refresh target / frame-time budget
Scene / fixture
Surface Class
Execution Tier
CPU render budget
GPU render budget
Memory / VRAM budget
Sustained-duration requirement
Thermal / power condition where relevant
Reference / Golden quality contract
```

## Baseline workloads

V1 must at minimum maintain stable workloads for:

- 1 Hero;
- 3 Hero;
- 32 Character;
- 100 Cel/Portable-execution instances;
- 500 Cel/Portable-execution instances;
- 1000 Cel/Portable-execution instances;
- Hero + skin + morph;
- Hero + outline;
- Hero + full post.

## Example record shape

```text
Target ID: android-mid-portable-100-v1
Device: <qualified device>
Backend: Vulkan
Resolution: <release-defined>
Scene: 100 Cel surfaces
Execution Tier: Portable
GPU budget: <release-defined>
CPU render budget: <release-defined>
Memory budget: <release-defined>
Sustained duration: <release-defined>
```

Values remain `Not qualified` until measured on the named target.

## Comparison rule

A change may claim a performance improvement only against:

1. the same workload;
2. the same device/backend or a clearly isolated comparable environment;
3. the same visual-quality contract;
4. recorded before/after measurements.

Related ADR: ADR-0029.
