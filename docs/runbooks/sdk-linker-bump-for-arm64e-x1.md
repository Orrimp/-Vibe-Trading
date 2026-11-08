---
slug: sdk-linker-bump-for-arm64e-x1
status: living
owner: orchestrator
updated: 2026-11-08
---

# Recipe — update the toolchain so `ld` understands `arm64e.x1`

Surfaced 2026-11-08, operator-approved the same day. Closes bug-log
[`#147`](../dev-notes/bug-log.md)'s real fix. **Not blocking** — the `SDKROOT` pin works and is
in both re-lock drivers.

## Why — and why a reinstall is NOT the fix

`ld` rejects four `.tbd` files in the 27.0 SDK, including `libSystem.tbd` and `libc.tbd`:

```
tapi error: malformed file in '…/MacOSX27.0.sdk/usr/lib/libSystem.tbd'
```

**The files are not malformed.** Measured 2026-11-08 — line 3 of 27.0's `libiconv.tbd` ends in a
trailing comma and the flow sequence continues on line 4:

```yaml
targets:         [ x86_64-macos, x86_64-maccatalyst, arm64e-macos, arm64e-maccatalyst,
                   arm64e.x1-macos, arm64e.x1-maccatalyst ]
```

26.5 declares the first four and closes the list. **27.0 adds two triples for a new `arm64e.x1`
sub-architecture**, `tbd-version: 4` is identical on both, and the YAML is well-formed. This
linker simply does not know that triple and reports the whole file as malformed:

```
Apple clang version 21.0.0 (clang-2100.1.1.101)
ld-1267, BUILD Jun  8 2026
```

So `sudo rm -rf /Library/Developer/CommandLineTools && xcode-select --install` — what `#147`
originally recommended — **reproduces the fault exactly** if it reinstalls the same 27.0 SDK
beside the same `ld-1267`. Only an update that bumps the **linker** helps.

Note also: `SDKs/MacOSX.sdk` already points at the sound `MacOSX26.5.sdk`, yet
`xcrun --show-sdk-path` returns `MacOSX27.0.sdk`. Repointing the symlink will not help either —
the selection happens above it.

## Command

Check what a newer CLT would give you before installing anything:

```bash
softwareupdate --list
```

Then, if a Command Line Tools update is offered:

```bash
softwareupdate --install "<the exact label softwareupdate printed>"
```

If none is offered, install a full Xcode from the App Store and point at it:

```bash
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
```

## Steps

1. Run the BEFORE check below and keep its output.
2. `softwareupdate --list`. Look for a "Command Line Tools" item whose version is **newer than
   the one that shipped ld-1267 (June 2026)**. If there is none, use the Xcode route.
3. Install it (password required), or switch to a full Xcode.
4. Run the AFTER check. Compare.

## Timing

~1 minute for the checks. The CLT update is a few GB; Xcode is ~10 GB and slower. One-time.

## Expected result — the before/after check

**BEFORE** (run this now, it is the baseline):

```bash
ld -v 2>&1 | head -1
env -u SDKROOT cargo build -p backtest --bin backtest 2>&1 | grep -c 'tapi error'
```

Today that prints `ld-1267 …` and a non-zero count.

**AFTER** — the fix worked iff BOTH change:

```bash
ld -v 2>&1 | head -1                                                   # a project number > 1267
env -u SDKROOT cargo build -p backtest --bin backtest 2>&1 | tail -3   # "Finished", no tapi error
```

The second is the one that matters. A newer `ld` that still cannot read `arm64e.x1` leaves the
count non-zero, and then the answer is "stay on the pin" — which costs nothing.

## Failure diagnosis

- **`softwareupdate --list` offers nothing and no Xcode is installed** — then there is no newer
  linker available yet. Stop; the pin is the answer until Apple ships one. Re-run the BEFORE
  check in a month.
- **`ld -v` is unchanged after the update** — the update did not include the linker. Check
  `xcode-select -p`: if it still points at `/Library/Developer/CommandLineTools` while you
  installed Xcode, run the `xcode-select --switch` above.
- **`tapi error` count drops but a different link error appears** — that is progress, not this
  bug; capture it and it gets its own entry.
- **Anything in the workspace now fails that passed before** — revert with
  `sudo xcode-select --switch /Library/Developer/CommandLineTools` and say so. The pin is
  unaffected either way.

## Cleanup

None needed. The `SDKROOT` pin in `scripts/relock/*.sh` is harmless once the default SDK links
cleanly — it only applies when `SDKROOT` is unset — and it stays as insurance. Remove it only
after the AFTER check passes twice on different days.
