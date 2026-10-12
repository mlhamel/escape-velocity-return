# Combat

[active] — first pass live (M1)

Real-time top-down 2D, in the spirit of EV.

- Shields absorb first, armor takes the spill. Armor is the hull — armor gone
  means destroyed (`evr-core` `ShipState`)
- Shields regenerate slowly in flight (4.0/s); armor does not repair
- Escape is a legitimate tactic; hyperjump fuel is a combat resource
- Death: lose cargo, keep credits. The yard builds you a new hull (3 s)
- PvP follows government zones (see pvp-policy.md)
- Server-authoritative: clients send intents, server resolves hits

Live weapon: Light Blaster (`content/outfits/light-blaster.ron`) — damage 6,
cooldown 0.25 s, projectile speed 420, ttl 1.2 s. Pirates fly Turncoats and
hunt players in their system; attacking a player in Confederation space earns
a fine and a Patrol Cutter on your tail (see pvp-policy.md).

Open questions:

- The EV energy juggle (one pool for weapons, shields, afterburner) — arrives
  with the outfit system
- More weapon classes: beams, turrets, missiles
- Ramming damage: yes/no?