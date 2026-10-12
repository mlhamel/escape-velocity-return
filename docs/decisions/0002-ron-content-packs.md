# ADR 0002 — Content packs in RON

- Status: accepted
- Date: 2026-10-11

## Context

EV's plug-in architecture kept it alive for years. We want content as text
data that agents and humans can diff, validate, and regenerate — and, for
shards, as swappable universes.

## Decision

Content lives in `content/` as RON 0.12 files: `manifest.ron` plus
`ships/`, `systems/`, `commodities/`, `outfits/`, `missions/`, `factions/`.
`evr-data` owns the serde structs — the structs are the schema. Loading
validates: manifest paths must resolve, ids must be unique.

## Consequences

Adding a ship is a data edit, not a code change (planned skills: add-ship,
add-system, content-lint). Shard operators can ship custom packs — the MMO
version of EV plug-ins. Schema changes are migrations in `evr-data`.