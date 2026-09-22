# ADR-0030: Runtime Capability Reconfiguration

- **Status:** Proposed
- **Decision owner:** Architecture
- **Date:** 2026-09-22

## Context

ADR-0022 froze capability handling into two phases: Stage A contributes device requests before `RenderDevice` creation, and Stage B resolves the runtime variant once in `StylizedPlugin::finish` from the actual device features/limits. No re-resolution path existed, so the scene-workbench session controller (R2, `docs/showcase/01-showcase-examples-fixtures.md`) refused any case whose tier exceeded the session's resolved tier and told hosts to start a new session.

That is overly strict. When the device already enabled the required features/limits at Stage A, changing the *requested* policy (surface class, execution tier, fallback policy) is a pure re-resolution against capabilities that have not changed. Only a change in Device features/limits themselves requires a new device session — ADR-0022's invariant ("runtime capability selection must never assume it can enable features after device creation") is untouched by this ADR.

## Options

1. Let hosts write `StylizedResolvedCapabilities` / `StylizedRenderCapabilities` directly with a hand-built variant.
2. A deferred message/event request processed by a system on the next frame.
3. An exclusive `World` transaction entry point that validates, re-resolves and applies atomically, plus a change notification message for observers.

## Decision

Adopt option 3. Option 1 is rejected: writing resolved resources by hand impersonates reinitialization, skips validation, and can disagree with the stored device snapshot. Option 2 is rejected as the *entry point*: the session coordinator's reconfiguration is a transaction whose result (applied vs refused) must be known synchronously before the dependent action (e.g. a scene switch) proceeds; a deferred message would force a multi-frame pending state and let interleaved frames act on a half-dependent session. Option 2's messages survive only as *notifications*: `StylizedCapabilitiesChanged` is emitted after a successful change so per-frame consumers and hosts can react.

The contract:

1. **In-session changeable:** `requested_class`, `requested_tier`, `fallback_policy`, and the asserted `requested_features` of a new `StylizedCapabilityConfig`, as long as every asserted feature is already enabled on the device and the deterministic resolver can satisfy the request against the Stage B snapshot.
2. **Requires a new device session:** any change to Device features/limits. Reconfiguration can never add a device feature; asserting a feature outside the Stage B `actual` snapshot is refused with the missing list.
3. **Snapshot re-resolution:** resolution reuses the same `resolve_with_report` pure function against the `actual` snapshot stored in `StylizedResolvedCapabilities` at Stage B. The device is never re-read mid-session; a new device implies a new session (and therefore a new snapshot).
4. **Atomic application:** on success the facade updates `StylizedCapabilityConfig`, `StylizedResolvedCapabilities` and (render feature) `StylizedRenderCapabilities`, plus the render crate's capability-derived state (`StylizedInstanceTransportMode`), inside a single exclusive `World` scope. On failure nothing is written: no partial application, resources stay byte-identical, and a structured `StylizedReconfigError` reports exactly the missing features and/or the failed resolution (`blocking_tier`, `missing_features`, `limits_satisfied`).
5. **Generation and notification:** a monotonically increasing `StylizedReconfigGeneration` resource bumps on every applied *change*; a `StylizedCapabilitiesChanged` message carries the generation, the previous and current variants and the fallback report. An idempotent request (resolving to the incumbent variant and config) succeeds without bumping the generation and without emitting the message.
6. **In-flight content policy** (from the init-time audit, see Evidence):
   - Screen-space outline reads `StylizedRenderCapabilities` per frame; upgrades take effect without any rebuild. On downgrade the sync system now strips the prepass components and despawn the overlay triangle it had granted, reverting the camera to hull-only outline.
   - `StylizedInstanceTransportMode` is recomputed in the same transaction. It is currently a provisional marker (ADR-0004 pending): no runtime system consumes it, because per-entity `StylizedInstanceSlot` components declare table usage. Existing slot components are not rewritten; content built after the change observes the new mode.
   - Material pipeline specialization keys contain no tier bits (verified for cel/outline/character/hero/screen-outline), so no pipeline rebuild is required on re-resolution.
   - Tier-fallback material parameters produced by the `*_params_for_variant` helpers are baked into material assets at content build time by hosts. In-flight content keeps its baked parameters; hosts rebuild content (a CaseDriver scene switch already does) to adopt the new variant. Reconfiguration never rewrites live material assets.
   - The dynamic-quality governor reads the resolved tier per frame and tightens its ceiling automatically.

The scene-workbench controller consumes this API: a tier refusal first attempts an in-session reconfiguration to the required tier, and only refuses when the API reports a device-level gap. The refusal reports whether reconfiguration was attempted, so hosts can distinguish "retried and refused — needs a new device session" from "no capability snapshot — cannot attempt".

## Evidence

Init-time audit (2026-09-22, working tree): the only capability-derived state cached at plugin init is `StylizedInstanceTransportMode` (`crates/bevy-stylized-render/src/lib.rs`, via `instance::select_transport`); all other consumers (`screen_outline::sync_screen_outline`, facade `dynamic_quality_system`) read the resolved capabilities per frame. No material `specialize` key or bind-group-data key carries tier information, so pipelines do not cache the variant.

Tests (this change): CPU tests prove snapshot-only re-resolution (no `RenderDevice` in the world), exact missing-feature reporting, failure atomicity (resources byte-identical), generation/notification semantics and idempotent no-op requests. GPU evidence tests (Metal) prove a Portable headless session captures `outline`, upgrades in-session to Standard, switches to `outline-advanced` through the controller and captures a screen-outline image that differs from the Portable hull-only fallback, and that asserting an unenabled device feature (Bindless, not requested at Stage A) is refused with the session state unchanged. Existing gates (showcase captures, regression golden cells, integration suites) are unchanged.

## Consequences

- The facade gains a purely additive public API: `StylizedReconfigGeneration`, `StylizedCapabilitiesChanged`, `StylizedReconfigError`, and a `World` extension method. No existing public item changes signature or semantics; ADR-0022's two-phase lifecycle and fail-closed behavior are preserved.
- The render crate gains an additive downgrade cleanup in `sync_screen_outline` and documents `StylizedInstanceTransportMode` as transaction-refreshed.
- The scene-runtime controller's `RefusalReason::TierRequired` gains a `reconfiguration_attempted` flag (serde-defaulted), and interactive hosts may now reach Standard-tier cases from a Portable-started session when the device allows it.
- Cross-tier pixel comparisons must record the reconfiguration generation as part of execution identity; a capture taken before and after a generation bump is not the same execution condition.

## Revisit trigger

Revisit when ADR-0004 lands the real instance-transport consumers (the transport mode stops being a marker and downgrade may need slot migration), when Bevy exposes a supported device re-creation path, or when a future render path caches variant-derived state at init that this transaction does not yet refresh.
