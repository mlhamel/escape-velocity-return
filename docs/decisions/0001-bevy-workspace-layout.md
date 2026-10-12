# ADR 0001 — Bevy + Cargo workspace layout

- Status: accepted
- Date: 2026-10-11

## Context

Agent-driven development of a 2D space trading MMO. We need fast iteration,
a testable simulation, and a client/server split that supports both offline
play and a networked shard.

## Decision

Bevy 0.19 (the latest line supported by bevy_replicon_renet2, see ADR-0005)
over a Cargo workspace with five crates:

- `evr-core`: headless simulation (economy, factions, combat math). No Bevy,
  no I/O. Runs in unit tests and at scale.
- `evr-data`: content pack loader + validation (RON).
- `evr-protocol`: wire intents/events shared by client and server.
- `evr-server`: authoritative simulation app — lib for embedding, bin for
  standalone.
- `evr-client`: Bevy desktop app.

## Consequences

Simulation is testable without a window or network; the client stays a thin
presentation layer. Crate names are `evr-*` because bare `core` collides with
Rust's built-in core library.