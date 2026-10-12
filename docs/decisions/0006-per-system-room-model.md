# ADR 0006 — Per-system room model

- Status: accepted
- Date: 2026-10-11

## Context

A shard holds dozens–hundreds of players across ~100 star systems.
Replicating the whole galaxy to everyone is wasteful, and combat only
happens where you are.

## Decision

Each star system is a replication room. Players receive state only for the
system they occupy. A hyperspace jump is a room migration. The economy and
faction war run as global services above the rooms. bevy_replicon's
room/visibility features map directly onto this.

## Consequences

Bandwidth scales with local population, not shard population. Systems are
natural candidates for per-system process isolation later — if a shard ever
needs to scale out, the door stays open without an architecture rewrite.