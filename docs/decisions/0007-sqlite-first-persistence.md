# ADR 0007 — SQLite-first persistence

- Status: accepted
- Date: 2026-10-11

## Context

The shard needs accounts, ships, credits, and economy state. Zero-ops
matters for a solo dev, and agent-driven tests need embeddable storage.

## Decision

SQLite for M3+, accessed through a storage trait defined in `evr-server`.
The trait boundary keeps a Postgres swap possible if a shard's write load
demands it. Offline mode uses the same storage locally (a save file).

## Consequences

No server infrastructure required for early shards. Migrations must be
written conservatively (SQLite's ALTER is limited). A Postgres move is a
storage impl swap, not an architecture change.