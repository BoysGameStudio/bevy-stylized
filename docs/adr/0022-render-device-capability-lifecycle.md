# ADR-0022: RenderDevice Capability Lifecycle

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-11

## Context

In Bevy 0.19.1, requested wgpu features/limits are selected through renderer creation settings before the RenderDevice exists. Once created, plugins can inspect the enabled device features/limits but cannot retroactively request a stronger device.

## Options

1. Resolve all capabilities after `StylizedPlugin` starts.
2. Require a maximal device unconditionally.
3. Split capability handling into pre-device request policy and post-device runtime resolution.

## Decision

Use a two-stage model.

**Stage A — renderer creation policy:** a helper/configuration API may contribute required/optional `WgpuSettings`/limits before Bevy creates the RenderDevice.

**Stage B — runtime resolution:** after device creation, NPR reads the actual `RenderDevice::features()` and `RenderDevice::limits()` and resolves only variants supported by the enabled device.

Runtime capability selection must never assume it can enable features after device creation.

## Evidence

Bevy 0.19.1 `WgpuSettings` documents features/limits as renderer-initialization inputs, while `RenderDevice::features()` / `limits()` expose the actual post-creation device capabilities.

## Consequences

The facade needs an initialization path that can configure renderer creation when optional high-end paths require additional features. Portable defaults must not require stronger limits than Bevy's selected baseline.

## Revisit trigger

Revisit when Bevy changes RenderDevice creation/configuration APIs.
