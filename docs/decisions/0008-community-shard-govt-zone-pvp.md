# ADR 0008 — Community shard scale + government-zone PvP

- Status: accepted
- Date: 2026-10-11

## Context

"MMO" spans everything from co-op servers to EVE. We needed a scale target
and a PvP policy that fits EV's DNA without alienating new traders.

## Decision

**Scale**: community shard — one persistent galaxy, dozens–hundreds
concurrent players.

**PvP**: government zones are the ruleset. Attacks in policed space trigger
NPC response plus faction penalties; pirate/contested systems are open PvP.
EV's government logic IS the PvP design.

## Consequences

Solo-dev tractable while keeping a real shared-universe feel. Govt-zone
rules make player piracy viable without making core space a slaughterhouse.
The architecture does not preclude larger scale later (see ADR-0006).