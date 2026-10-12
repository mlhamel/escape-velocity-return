# ADR 0004 — Server-authoritative with embedded offline mode

- Status: accepted
- Date: 2026-10-11

## Context

The game must feel like single-player EV offline AND run as a shared shard
online. Two simulation code paths would double every bug and halve every
test's meaning.

## Decision

The `evr-server` library is authoritative in both modes:

- **offline**: the client runs the server lib in-process over an in-memory
  transport
- **online**: the same lib runs as a standalone server process

Clients never mutate authoritative state; they send intents
(`evr-protocol`).

## Consequences

One simulation code path; headless tests cover online behavior; the client
needs no mode-specific simulation logic. Offline saves are local files;
online state lives on the shard. Online-only features (shared economy,
metagame) degrade to local simulation when offline.