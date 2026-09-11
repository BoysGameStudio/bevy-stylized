# ADR-0025: Profile API and Schema Stability

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-11

## Context

Rust does not provide a stable binary ABI for arbitrary Rust-to-Rust plugin interfaces. Calling a Rust trait contract a stable ABI would overpromise binary compatibility.

## Options

1. Promise a stable Rust ABI.
2. Use a stable Rust source API plus versioned serialized profile schemas.
3. Introduce an explicit C/WIT/component ABI immediately.

## Decision

For the planned crate-based ecosystem, promise a versioned Rust **API** and versioned **schema contracts**, not a Rust ABI.

Binary plugin ABI is out of scope unless a future version explicitly adopts an ABI technology (for example C ABI, WIT/component model, or another deliberately versioned binary interface).

## Evidence

Rust's default type layout and calling conventions are not a general stable binary interface. The current ecosystem plan distributes profiles as Rust crates and data/schema packages, so source/API compatibility is the relevant contract.

## Consequences

V3 documentation uses `Stable Profile API + Versioned Schema`. SemVer and migration tests govern source/schema compatibility. Dynamic binary plugins require a new ADR.

## Revisit trigger

Revisit if third-party profiles must be loaded dynamically without recompiling the host application.
