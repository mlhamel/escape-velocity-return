# Player Identity (MMO)

[active] — implemented (M3): username/password login, argon2 hashing, SQLite persistence

- One named identity per shard; the name IS the reputation
- Accounts: username/password on the shard server (argon2 hashing, no email
  at first)
- Offline single-player identity is local-only (embedded server, no account)
- Names: unique per shard; no renames — reputation is sticky by design