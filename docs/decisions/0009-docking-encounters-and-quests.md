# ADR 0009 — Docking encounters with planet chiefs and quests as content

- Status: accepted
- Date: 2026-10-16

## Context

In Escape Velocity (1996), missions lived in spaceport bars and mission
computers — separate sub-menus the player had to click through after landing.
In our single-screen top-down HUD, buried menus dilute the moment of arrival
and hide content from new players and MCP agents alike.

We want docking at a world to feel immediate and characterful: a local
authority greets you on touchdown, offers a deal, demands a cut, or puts work
on the table.

## Decision

1. **Planet chiefs on dock**: `PlanetDef` gains an optional `chief: Option<ChiefDef>`
   block (name, portrait path, and character-voiced dialog lines). When a
   player docks at a planet with a chief, the server rolls an encounter and
   attaches a replicated `ChiefEncounter` component to the player entity.
2. **Replicated component, not one-shot event**: `ChiefEncounter` and
   `ActiveQuests` are replicated components (`crates/protocol`) rather than
   transient server events so they survive client/MCP idle reconnects without
   special replay plumbing.
3. **Quests as a first-class content resource**: Quests live in
   `content/quests/*.ron` and are listed under `quests: []` in
   `content/manifest.ron`. `QuestDef` covers delivery, bounty, and optional
   timed (`deadline_secs`) variants. Cross-references to systems, planets, and
   commodities are validated at pack load time.
4. **Coexistence with market trading**: The chief encounter renders as a side
   overlay panel (dismissible with `Esc`, reopenable with `T` while docked).
   When open, digit keys route to encounter choices; when dismissed, digit keys
   return to commodity trading.
5. **Persistence**: Active and completed quest state persists in SQLite alongside
   player state (`active_quests_json`). Ship destruction fails active quests.

## Consequences

Every inhabited world has a face and voice without adding modal menu screens.
Adding a quest or a chief is pure RON data plus a portrait PNG, validated by
`cargo test -p evr-data`. Because encounters and active quests sit on the
player entity as replicated state, both the Bevy desktop client and `evr-mcp`
observe and respond to them through the same authoritative protocol.
