# Escape Velocity: The Return

Community-shard MMO inspired by Escape Velocity (1996): 2D top-down space
trading, missions, and combat in a shared persistent galaxy. Rust + Bevy.
Server-authoritative; the same server code embeds in the client for offline
single-player.

## Commands

- build: `cargo build --workspace`
- test: `cargo test --workspace` (single crate: `cargo test -p evr-core`)
- lint: `cargo clippy --workspace -- -D warnings`
- format: `cargo fmt --all` (check: `cargo fmt --all --check`)
- run server: `cargo run -p evr-server` (headless, runs from workspace root)
- run client: `cargo run -p evr-client` (desktop session required)

## Structure

- `crates/core` — headless simulation (economy, factions, combat math); no Bevy, no I/O
- `crates/data` — RON content loader + validation; its structs are the content schema
- `crates/protocol` — client intents / server events; shared wire types
- `crates/server` — authoritative simulation app (embeddable lib + standalone bin)
- `crates/client` — Bevy desktop app
- `content/` — RON content packs; data, never code
- `docs/gdd/` — canonical game design; README is the index
- `docs/decisions/` — ADRs; write one before non-trivial architecture changes
- `docs/ideas/` — idea inbox; status-tagged, never deleted
- `docs/reference/` — EV/EVO/EVN/Endless Sky research
- `prompts/` — curated prompt library
- `.opencode/skills/` — repeatable project workflows
- `ops/` — deployment configs (M3+)

## Conventions

- Content is data in `content/`, never code. Use the add-ship/add-system skills.
- Server is authoritative. Clients send intents (`crates/protocol`); they never
  mutate game state.
- Offline play = embedded server over in-process transport; no second code path.
- `evr-core` stays Bevy-free and I/O-free. Presentation belongs in `evr-client`.
- Bevy/replicon/renet2 versions are pinned together (read docs/decisions/0005
  before upgrading). rustc >= 1.95 required by bevy 0.19.
- Client bevy features exclude audio and gamepads: this machine lacks ALSA
  and libudev dev headers. Re-enable after installing `libasound2-dev` and
  `libudev-dev`.
- In-game text follows `docs/gdd/writing-style.md`.
- History lives in git: ADRs for decisions, `docs/ideas/` for everything considered.
  Never delete an idea; re-status it.
- No unsafe code (workspace lint).