# ADR 0005 — Netcode: bevy_replicon + renet2

- Status: accepted
- Date: 2026-10-11

## Context

We need Bevy-native, server-authoritative replication with a maintained
transport layer.

## Decision

bevy_replicon 0.41 + bevy_replicon_renet2 0.19.1 over bevy 0.19 — the
coherent published pairing. Verified on crates.io: bevy_replicon_renet2
0.19.1 requires bevy ^0.19 and bevy_replicon ^0.41. Newer bevy_replicon
releases exist (0.44 line pairs with bevy ^0.19; 0.45, released
2026-10-11, already moved to bevy ^0.20) but none has a matching renet2
integration yet — pinning ahead of the integration crate would silently
duplicate bevy_replicon in the dependency tree.

Pins live in the root `Cargo.toml` `[workspace.dependencies]`. Re-pin the
whole set together when upgrading, and check bevy_replicon_renet2 first —
it trails bevy_replicon releases.

## Consequences

Replication is declarative (events + components); renet2 provides the
netcode (authentication, encryption, reliability). The server bin attaches UDP (native_transport) at M2. The server has run
slim (std, multi_threaded, bevy_state, serialize — no render features) since
the M0 ALSA fix.