# ADR 0003 — Monorepo with in-repo docs

- Status: accepted
- Date: 2026-10-11

## Context

Solo dev + AI agents. Everything agents touch (code, content, design,
prompts, skills) should share one git history and be reachable from
AGENTS.md as the entry point.

## Decision

Single repo. Canonical doc homes:

- `docs/gdd/` — design truth (README is the index)
- `docs/decisions/` — ADRs (this)
- `docs/ideas/` — idea inbox, status-tagged, never deleted
- `docs/reference/` — EV/EVO/EVN/Endless Sky research
- `docs/diagrams/` — Mermaid architecture diagrams
- `prompts/` — curated prompt library
- `.opencode/skills/` — repeatable project workflows
- `ops/` — deployment (M3+)

Obsidian stays for session notes (`/session-handoff`) only.

## Consequences

Every design change is a reviewable diff. History = git + ADRs + idea
statuses. If a plug-in ecosystem emerges, `content/` can split into its own
repo without structural changes.