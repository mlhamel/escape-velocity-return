---
name: add-quest
description: Add a new quest definition (or planet chief) to content/ and validate the pack
---

# Add a quest

1. Create `content/quests/<id>.ron`:
   - Fields: `id`, `name`, `giver: ("<system-id>", "<planet-id>")`, `kind`,
     `reward_credits`, `description`
   - `kind` is one of:
     - `Delivery(commodity: "<id>", qty: <u32>, destination: ("<system-id>", "<planet-id>"), deadline_secs: Option<u64>)`
     - `Bounty(system: "<system-id>", target: "<ship-id>", deadline_secs: Option<u64>)`
   - `deadline_secs` is optional (`#[serde(default)]`); omit it for untimed
     contracts or set `deadline_secs: Some(180)` for timed runs
   - `giver` planet should have a `chief: Some((...))` block in its system RON
     file so the player can receive the quest on docking
   - `description` follows `docs/gdd/writing-style.md` (character-voiced, dry
     wit, ≤120 words)
2. Add `"quests/<id>.ron"` to `quests:` in `content/manifest.ron`
3. If adding a new planet chief portrait, place a 128×128 RGBA PNG at
   `assets/textures/chiefs/<planet-id>.png` (or add a row in
   `ops/generate_placeholder_art.py` and run it)
4. Validate: `cargo test -p evr-data` (checks manifest paths, unique quest ids,
   giver/destination planet references, target system references, and chief
   portrait assets)
5. Verify: `cargo clippy --workspace -- -D warnings`
