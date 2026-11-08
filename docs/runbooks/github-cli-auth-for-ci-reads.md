---
slug: github-cli-auth-for-ci-reads
status: living
owner: orchestrator
updated: 2026-11-08
---

# Recipe — authenticate `gh` for github.com so CI failures become readable

Surfaced 2026-11-08, operator-approved the same day. Blocks: bug-log
[`#172`](../dev-notes/bug-log.md) and every future CI red.

## Why

`gh` on this machine is authenticated to **git.dhl.com** and **dhl.ghe.com** only. For
`github.com` it is unauthenticated, which means:

- `gh run view <id> --log-failed` → **HTTP 403** *"Must have admin rights to Repository"*.
- The documented fallback, check-run **annotations**, returned **HTML / HTTP 504** on both
  attempts on 2026-11-08 — so it is not reliable either.
- Unauthenticated REST is capped at **60 requests per hour by IP**, which a single polling
  watch once consumed entirely.

The most that can be established today is *which step* failed (three requests, recipe at the
bottom). Not *what it asserted*. So a red CI cannot be diagnosed, only narrated — which is why
`#172` is recorded as an observation rather than a finding.

## Command

```bash
gh auth login -h github.com
```

## Steps

1. Run the command above in a terminal on this machine.
2. Choose **GitHub.com** when asked for the host (if it does not ask, the `-h` flag already
   answered it).
3. Choose **HTTPS** for the git protocol. Say **No** to "authenticate Git with your GitHub
   credentials" — git already uses the SSH remote (`git@github.com:Orrimp/-Vibe-Trading.git`)
   and this must not change it.
4. Choose **Login with a web browser**, copy the one-time code it prints, press Enter, and
   paste the code in the browser page that opens.
5. Approve the requested scopes. `repo` is sufficient; `read:org` is harmless if offered.

## Timing

~1 minute, one browser round-trip. One-time — the token persists in `gh`'s keyring.

## Expected result

```bash
gh auth status
```

lists **github.com** alongside git.dhl.com and dhl.ghe.com, with `- Active account: true` under
it. Then this, which is 403 today, prints the failing assertion:

```bash
gh run view 42463577247 --log-failed | tail -40
```

## Failure diagnosis

- **"error connecting to github.com"** — a proxy or VPN is intercepting; retry off the VPN.
- **The browser flow completes but `gh auth status` still omits github.com** — the keyring write
  failed. Re-run with `gh auth login -h github.com --with-token < <(pbpaste)` after copying a
  Personal Access Token (classic, scope `repo`) from
  <https://github.com/settings/tokens>.
- **`--log-failed` still 403 after authenticating** — then it genuinely needs repo-admin rights
  rather than any token, and the three-request step-name recipe below stays the ceiling. Say so
  and I will stop proposing this.
- **Rate limit still 60/hour** — the token was not picked up; `gh api rate_limit` is exempt from
  the limit and will report misleading numbers, so check with a real request instead.

## Cleanup

None. Nothing is installed and no repository state changes. To undo:
`gh auth logout -h github.com`.

## What works WITHOUT this (the current ceiling)

Three requests take you from "CI is red" to the failing step's name. Per-step conclusions live
inside `--json jobs`, which needs no credential:

```bash
ID=$(gh run list --limit 10 --json databaseId,headSha \
      --jq '.[] | select(.headSha | startswith("637c6998")) | .databaseId')
gh run view "$ID" --json jobs --jq '.jobs[] | "\(.conclusion)  \(.name)"'
gh run view "$ID" --json jobs \
  --jq '.jobs[] | select(.conclusion=="failure") | .steps[]
        | select(.conclusion!="success") | "\(.conclusion)  step \(.number): \(.name)"'
```
