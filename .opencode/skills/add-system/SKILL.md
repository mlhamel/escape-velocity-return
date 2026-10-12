---
name: add-system
description: Add a new star system with planets, commodities, and hyperlinks
---

# Add a star system

1. Create `content/systems/<id>.ron`:
   - Fields: id, name, government, description, position ((x, y) galaxy
     coordinates), links, planets
   - Each planet: id, name, description, commodities `[(id, price)]`
   - Descriptions follow docs/gdd/writing-style.md
2. Add `"systems/<id>.ron"` to `systems:` in `content/manifest.ron`
3. Update the links of every connected system to point back — links must be
   reciprocal or pack validation fails
4. Prices: design a profitable loop against existing systems
   (see docs/gdd/economy.md)
5. NPC presence is wired in `crates/server/src/gameplay.rs`, not in content
   (for now)
6. Validate: `cargo test -p evr-data`
7. Verify: `cargo test -p evr-server` (the embedded slice test exercises the
   whole pack end-to-end)