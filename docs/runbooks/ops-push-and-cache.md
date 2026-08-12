---
slug: ops-push-and-cache
status: living
owner: orchestrator
updated: 2026-07-09
---

# Ops runbook — push transport + cargo cache recovery

Remediation **P0**. The two recurring operational wedges that ate real session time,
with their permanent fixes and fast recoveries.

## 1. The push wedge (1Password SSH agent relock)

**Symptom** (recurred ~every push in the 2026-07 sessions):

```
sign_and_send_pubkey: signing failed for ED25519 "Github SSH Key" from agent:
communication with agent failed
git@github.com: Permission denied (publickey).
```

**Root cause:** the repo's remote authenticates through the 1Password SSH agent
(`SSH_AUTH_SOCK` → 1Password). A **locked vault still serves `ssh-add -l`** (cached
key *list*) but **refuses signing** — so probes look alive while every push/fetch
fails or hangs. The vault auto-relocks within seconds of an unlock, so
agent-toggling races almost always lose.

**Diagnosis (5 seconds):** `ssh-add -l` showing the key proves nothing. The real
test is a signing operation: `ssh -o ConnectTimeout=5 -o BatchMode=yes -T
git@github.com` — `Permission denied (publickey)` + the `communication with agent
failed` line = locked vault, not a key problem.

**Permanent fix (choose one):**

- **(a) Repo-scoped deploy key outside the vault (recommended).** A dedicated
  ED25519 key on disk + macOS keychain, used *only* for this repo's remote, so
  pushes never depend on the vault being unlocked. Setup recipe: see the operator
  recipe in the P0 kickoff message (also reproduced below).
- **(b) HTTPS + credential helper.** `git remote set-url origin
  https://github.com/Orrimp/-Vibe-Trading.git` + `git config credential.helper
  osxkeychain` + a PAT entered ONCE interactively at first push (never through an
  agent chat — secrets-in-chat is off-limits).

### (a) Deploy-key recipe (operator, one-time, ~3 min)

```bash
# 1. Generate a repo-only key (no passphrase, or passphrase in Apple keychain):
ssh-keygen -t ed25519 -f ~/.ssh/id_ed25519_trading -C "trading-deploy" -N ""

# 2. Add the PUBLIC key to GitHub: repo → Settings → Deploy keys →
#    "Add deploy key" → paste ~/.ssh/id_ed25519_trading.pub → allow WRITE access.

# 3. Scope it to this remote only (~/.ssh/config):
cat >> ~/.ssh/config <<'EOF'
Host github-trading
  HostName github.com
  User git
  IdentityFile ~/.ssh/id_ed25519_trading
  IdentitiesOnly yes
EOF

# 4. Point the repo at the scoped host:
cd ~/Projects/Privat/trading/trading
git remote set-url origin github-trading:Orrimp/-Vibe-Trading.git

# 5. Verify (expect "successfully authenticated"):
ssh -T github-trading
git push --dry-run origin main
```

**Expected result:** pushes succeed regardless of 1Password vault state.
**Failure diagnosis:** `Permission denied` at step 5 → the deploy key wasn't added
with write access, or `IdentitiesOnly` isn't picking the file (check `ssh -vT
github-trading | grep identity`). **Cleanup:** none; the old vault key remains
valid for other repos.

### Interim workaround (until the fix lands)

Push from the operator's own interactive terminal (`git push origin main`) — the
1Password approval prompt surfaces in a foreground session and one click lands it.
Orchestrator pushes only work inside a fresh unlock window.

**Commit signing:** the same vault wedge blocks GPG/SSH commit signing — commit
with `--no-gpg-sign` when wedged (established practice this project).

## 2. Cargo cache corruption (stale incremental locks)

**Symptom:** builds hang past the timeout or fail with corrupt-artifact errors;
stuck `rustc` processes linger.

**Recovery (in order, escalate only as needed):**

```bash
# 1. Kill stuck compiler processes:
pkill -9 rustc; pkill -9 'cargo build'

# 2. Drop ONLY the incremental caches (cheap, usually sufficient):
rm -rf target/debug/incremental target/release/incremental

# 3. Full clean (expensive — ~full-workspace rebuild follows):
cargo clean
```

**Prevention:** don't run two cargo builds against the same target dir
concurrently (background build + foreground test is the usual trigger; the
orchestrator serializes them).

## 3. Local governance hooks (pre-commit gates)

**Since 2026-07-27** the repo ships committed hooks under `.githooks/`. The
`pre-commit` hook runs the two constitutional checks — `scripts/spec_lint.py`
(ADR-0082 triad) and `scripts/verify_anchors.sh` (AD-2, 119/119) — in
~seconds, no cargo. Until then these gates ran only when an agent remembered
(bug-log #66 documents the cost of discipline-only enforcement). CI runs the
same pair on the ubuntu leg ("Governance gates" step), so a bypassed hook is
still caught at push.

**Enable once per clone** (hooksPath is not clonable state):

```bash
git config core.hooksPath .githooks
```

**Bypass in an emergency:** `git commit --no-verify` (CI still gates).

## Changelog

- 2026-07-09 (orchestrator): created — remediation P0; consolidates the push-wedge
  root cause (vault-locked signing), the deploy-key/HTTPS permanent fixes, the
  probe-then-push interim, and the cargo incremental-cache recovery ladder.
- 2026-07-27 (orchestrator): § 3 added (committed `.githooks/` + CI governance
  gates). Wedge status note: a plain `git push` succeeded 2026-07-27 (wedge
  clear at that moment); the deploy-key fallback remains the resilience
  recommendation (board action item, owner: operator).
- 2026-08-11 (orchestrator): **wedge RECURRED** blocking the 1-20 close-out push
  (commit `8298697`). Symptom reproduced verbatim per § 1, and the § 1 diagnosis
  held exactly: `ssh-add -l` listed both ED25519 keys while `ssh -T git@github.com`
  returned `sign_and_send_pubkey: signing failed … communication with agent failed`
  → locked vault, not a key problem. **No retry loop was attempted** (§ 1 records
  that the vault auto-relocks within seconds, so agent-toggling races lose), and
  the permanent fix was NOT half-applied: recipe (a) step 2 adds a deploy key to
  GitHub repo settings, which is an operator-owned account change. Commits are
  safe on local `main` and accumulate without loss — the burn-down continued.
  **RESOLVED ~06:56 the same night without intervention** — a later `git push`
  succeeded and carried all three queued commits (`8298697`, `46dd71d`,
  `ed36766`) in one go, consistent with the vault having been unlocked in the
  interim. Nothing was lost by waiting, which is the point worth keeping: the
  correct response to this wedge is to keep committing locally and retry later,
  NOT to fight the agent.
  **Standing status: the permanent fix (a) or (b) has still never been applied**;
  this is now the third recorded recurrence. Each time it has cleared on its own,
  which is exactly why it keeps not getting fixed. Owner: operator.
