# Combat

[draft]

Real-time top-down 2D, in the spirit of EV.

- Single energy pool: weapons fire, shield recharge, and afterburner all draw
  from it — the juggle IS the combat
- Shields absorb first, armor takes the spill (see `evr-core` `ShipState`)
- Escape is a legitimate tactic; hyperjump fuel is a combat resource
- PvP follows government zones (see pvp-policy.md)
- Server-authoritative: clients send intents, server resolves hits

Open questions:

- Projectile vs instant-hit model per weapon class
- Shield/armor/energy regeneration curves
- Ramming damage: yes/no?