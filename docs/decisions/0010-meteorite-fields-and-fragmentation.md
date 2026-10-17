# ADR 0010 — Meteorite fields, rotational drift, ship collisions, and binary fragmentation

- Status: accepted
- Date: 2026-10-17

## Context

Space in between planets and jump boundaries currently contains only ships and
projectiles. Adding meteorite fields introduces environmental navigation
hazards, target practice, and dynamic spatial obstacles that make flying
through a star system tactile even outside of pirate ambushes.

We want meteorites that:
1. Appear in diverse sizes (`Large`, `Medium`, `Small`).
2. Either remain stationary, drift slowly, or streak quickly in varied
   directions.
3. Spin on their own axis (`Heading`) whenever they are moving.
4. Damage any undocked ship they crash into.
5. Can be shot by ship weapons and recursively split into **2 parts**, then
   **2 parts** again, and finally **disappear** when the smallest fragments
   are destroyed.

## Decision

1. **Authoritative replicated `Meteorite` entity (`crates/protocol`)**:
   Meteorites are server-authoritative entities with `Position`, `Heading`,
   `SystemMembership`, and a replicated `Meteorite` component carrying:
   - `size: MeteoriteSize` (`Large`, `Medium`, `Small`)
   - `hp: f32` and `max_hp: f32`
   - `spin_rate: f32` (radians/sec; `0.0` when stationary, non-zero when moving
     so moving meteorites rotate on themselves)
   Server-side physics tracks linear `Velocity(Vec2)` and a per-ship collision
   cooldown so a single grazing impact does not apply damage on every tick.

2. **Three speed profiles (`Stationary`, `Slow`, `Fast`)**:
   - **Stationary**: zero linear velocity and zero `spin_rate`.
   - **Slow**: `15..45` world units/sec in a random direction, slow self-spin.
   - **Fast**: `70..140` world units/sec in a random direction, faster self-spin.
   Moving meteorites wrap across the system field boundary so the belt stays
   populated near the play area.

3. **Ship collision damage (undocked only)**:
   When an undocked ship (`Without<DockedAt>`) overlaps a meteorite
   (`distance <= SHIP_RADIUS + meteorite.radius()`), the server applies
   size-scaled collision damage via `evr_core::combat::apply_damage`, emits
   `ShipDamaged` (and `ShipDestroyed` if lethal), and imparts a separation
   impulse so the ship and rock rebound cleanly. Docked ships are immune.

4. **Projectile hits & recursive binary fragmentation (`1 -> 2 -> 4 -> 0`)**:
   Projectiles check collisions against both undocked ships and meteorites in
   the same system. When a meteorite's `hp` reaches `<= 0.0`:
   - `Large` spawns **2 `Medium`** fragments at slight lateral offsets, with
     velocity vectors diverging by `±40°` and opposite spin directions.
   - `Medium` spawns **2 `Small`** fragments with the same binary split rule.
   - `Small` despawns completely (`ShipDestroyed`-style visual debris flash on
     the client).

5. **Content-driven per system (`crates/data`)**:
   `SystemDef` in `content/systems/*.ron` gains an optional
   `#[serde(default)] meteorites: Option<MeteoriteFieldDef>` block defining the
   system's meteorite count, belt radius, and size/speed weights.

## Consequences

Meteorite fields use the exact same server-authoritative replication pipeline
as ships and planets: offline single-player, online multiplayer shards, and
`evr-mcp` agents all see identical meteorite positions, sizes, and splits.
Recursive binary splitting (`Large -> 2 Medium -> 4 Small -> 0`) is bounded at
a maximum of 4 active leaf fragments per root meteorite, keeping entity counts
predictable in benchmark scenarios.
