# bug-log #131 — the anchor-literal reconciler's own scope was the unstated assumption

**Date:** 2026-09-28
**Author:** sub-agent (dev seam), under the orchestrator's #131 brief
**Subject:** `scripts/check_determinism_anchors.py` scanned ONE file and reported a total as if
it covered the tree. Six anchor-SHA literals in four other test files were never reconciled
against `evidence/anchors.toml`.
**Scope of this change:** `scripts/check_determinism_anchors.py` + comment/declaration lines in
four `crates/backtest/tests/*.rs` files. No `evidence/`, no `_bmad-output/`, no engine code, no
anchor value changed anywhere.
**Gates:** `ANCHORS PASS (119 / 119) [declared 119 · floor 119]` before **and** after.
`--self-test` 9/9 probes fired. Four test binaries compile and pass (see § Verification).

---

## 1. The finding

The tool printed, truthfully:

```
check_determinism_anchors: OK — 29 of 29 resolved literal(s) match ... Sites seen: 29 (floor 29)
```

29 of 29 **in `crates/backtest/tests/determinism.rs`**, because `main()` scanned the single
`DETERMINISM_RS` constant. The sentence is true and the number is right; what is missing is the
qualifier. Derived mechanically —

```
grep -rln 'const [A-Z_]*ANCHOR[A-Z_]*\s*:\s*&str' crates/ --include='*.rs'
```

— the repo holds **35** such literals in **five** files. So the gate that exists to catch "a
number nobody has checked" was itself reporting a number whose scope nobody had checked: the
same defect, one level up.

Two compounding details:

* The digest `076929bb63d9bec0…` is written out in **three** places — `evidence/anchors.toml`
  row 69, `run_yahoo_sma_ticker_flag.rs`, and `reproducibility_sample_figure.rs` — and nothing
  checked that the three agreed.
* The name grep also matches `crates/ui/src/strings.rs`'s
  `STRATEGY_REGISTRY_LAST_ANCHOR_PREFIX: &str = "Anchor: "`, which is a UI label, not a digest.
  The correct filter is the **value** shape, not the name; that is now what the tool uses, and
  the `--self-test` P6 probe fails if it ever stops being.

## 2. Before / after scope

| file | anchor literals | reconciled BEFORE | reconciled AFTER |
|---|---|---|---|
| `crates/backtest/tests/determinism.rs` | 29 | 29 | 29 |
| `crates/backtest/tests/multi_pair_determinism.rs` | 2 | **0** | 2 |
| `crates/backtest/tests/run_yahoo_sma_ticker_flag.rs` | 2 | **0** | 2 |
| `crates/backtest/tests/theta_surface_reproduction.rs` | 1 | **0** | 1 |
| `crates/backtest/tests/reproducibility_sample_figure.rs` | 1 | **0** | 1 |
| **total** | **35** | **29** | **35** |
| `crates/ui/src/strings.rs` (name-grep match) | 0 — value is `"Anchor: "` | n/a | excluded by the hex-value filter, deliberately |

New summary line, verbatim:

```
check_determinism_anchors: OK — 35 of 35 resolved literal(s) match (12 canonical
v5-realdata-medium-2026-05, 17 declared-ns, 6 synthetic; 0 skipped: cfg-gated; 0 unresolved).
Sites seen: 35 across 5 file(s) (floor 35) — determinism.rs 29 · multi_pair_determinism.rs 2 ·
run_yahoo_sma_ticker_flag.rs 2 · theta_surface_reproduction.rs 1 ·
reproducibility_sample_figure.rs 1.
```

(printed on one line; wrapped here). The per-file tail is the point: a file dropping out of
scope is now visible in the output, not only in a total. A file listed in `SCANNED_FILES` that
yields **zero** literals is a hard failure, and a listed file that no longer exists raises
rather than contributing a silent zero — the pre-#131 tool could not tell "this file has no
literals" from "this file is never opened".

The `declared-ns` column is also new. The old line folded all 17 namespace-declaring sites into
"canonical v5-realdata-medium-2026-05"; 17 of them are **not** that row.

## 3. Mismatches found: ZERO

**Every one of the six previously-unreconciled literals already agreed with its
`evidence/anchors.toml` row.** Reported as a clean result, because a clean result is a finding:
the six gates were correct, they were merely unguarded. Had one been wrong, it would have been
wrong silently for as long as the file existed.

| file:line | const | scenario | anchors.toml `version` | digest | verdict |
|---|---|---|---|---|---|
| `multi_pair_determinism.rs:144` | `ANCHOR` | `pairs-2023-zscore-mr` | `v1.5a + v5-realdata-medium-2026-05` | `ac647a59…` | match |
| `multi_pair_determinism.rs:155` | `ANCHOR` | `pairs-2024-h1-zscore-mr` | `v1.5a + v5-realdata-medium-2026-05` | `5bee5e9c…` | match |
| `run_yahoo_sma_ticker_flag.rs:154` | `BTC_ANCHOR_SHA` | `btc-yahoo-2024-1d-sma-cross` | `lab-yahoo-realdata-v0.1.1` | `076929bb…` | match |
| `run_yahoo_sma_ticker_flag.rs:170` | `ETH_ANCHOR_SHA` | `eth-yahoo-2024-1d-sma-cross` | `lab-yahoo-realdata-v0.1.3` | `c854ff2b…` | match |
| `theta_surface_reproduction.rs:362` | `ANCHOR` | `v1-momentum-2023-block-bootstrap-real-fy-mc` | `mc-robustness-2026-06` | `3aae06c0…` | match |
| `reproducibility_sample_figure.rs:49` | `ANCHORED_BODY_SHA` | `btc-yahoo-2024-1d-sma-cross` | `lab-yahoo-realdata-v0.1.1` | `076929bb…` | match |

Independently corroborated at runtime, not only statically: `multi_pair_determinism`'s two
reproduction gates, `run_yahoo_sma_ticker_flag`'s `btc_default_sha_matches_anchor_69` /
`eth_ticker_sha_matches_anchor_70`, and
`reproducibility_sample_figure::the_sample_corpus_reproduces_the_anchored_figure` all RAN (not
skipped) and passed — so five of the six digests are live-reproduced today as well as
statically reconciled. The sixth (`3aae06c0…`, the Monte-Carlo row) stays `#[ignore]`d and
corpus-gated at ~4 min; it is statically reconciled only, which is what it was before.

### 3.1 Namespace derivation, and one deviation from the brief

Each new site's target row was read out of `evidence/anchors.toml`, not taken from the brief:

* `btc-yahoo-2024-1d-sma-cross` → `lab-yahoo-realdata-v0.1.1` (used by **two** sites)
* `eth-yahoo-2024-1d-sma-cross` → `lab-yahoo-realdata-v0.1.3`
* `v1-momentum-2023-block-bootstrap-real-fy-mc` → `mc-robustness-2026-06`
* `pairs-2023-zscore-mr`, `pairs-2024-h1-zscore-mr` → `v1.5a + v5-realdata-medium-2026-05`

**The brief said each new site needs a `// anchor-ns:` declaration because none is the canonical
suffix. For the two `pairs-*` sites that is not what the file says, and they got no
declaration.** Their version string *contains* `v5-realdata-medium-2026-05`, so the default
resolution finds them, and finds them **uniquely** — the sibling row is `v1.5a + noop-baseline`,
which does not contain the suffix. Declaring one would also have been actively wrong: the
`// anchor-ns:` value is a single non-space token (`(\S+)`), so `v1.5a + v5-realdata-medium-2026-05`
cannot be written as one; and the token that *would* fit, `v1.5a`, matches **both** rows, which
`sha_for()` treats as ambiguous → a hard error. The two sites carry a comment recording that
the absence of a declaration is deliberate.

## 4. Other findings, all fixed in this change

1. **Stale documentation, `run_yahoo_sma_ticker_flag.rs`.** The `ETH_ANCHOR_SHA` doc comment and
   the `eth_ticker_sha_matches_anchor_70` assert message both claimed anchors.toml row 70 still
   held the superseded v0.1.2 SHA `e59a5f87…`, and that bulk re-emit was "deferred to v0.1.4 BNB
   ship". Both statements are false: the v0.1.4 ship was retired 2026-06-16, and row 70 was
   re-emitted under ADR-0038 § D6.b earlier today (commit `b079822f`) to
   `lab-yahoo-realdata-v0.1.3` = `c854ff2b…`, the value the constant already carried. Rewritten
   to say what is true. This is exactly the drift the reconciler cannot see — it compares
   digests, not prose — so it is recorded here rather than treated as covered.
2. **File-scope consts inherited an unrelated fn's `#[cfg(feature = …)]`.** `current_fn` is
   sticky, and both `run_yahoo_sma_ticker_flag.rs` constants sit below
   `#[cfg(feature = "yahoo")] fn pinned_table_allowed_yahoo_tickers_matches_data_crate`. On the
   first broadening they were therefore classed "skipped: cfg-gated" — the literals this whole
   change exists to check, dropped at the last step and counted as fine. A site at brace depth 0
   is now `(file scope)` with `cfg_gated=False`.
3. **Declaration bleed between file-scope consts.** The backwards declaration window stopped
   only at a `fn` boundary. Two consts in a row at file scope have no `fn` between them, so the
   second inherited the first's `// anchor-scenario:` **and** `// anchor-ns:` — `ETH_ANCHOR_SHA`
   resolved to `btc-yahoo-2024-1d-sma-cross`. Caught by probe P7 while writing it. The window
   now also stops at the previous anchor const.
4. **The broadened name regex could not match a bare `ANCHOR`.** `[A-Za-z_][A-Za-z0-9_]*ANCHOR…`
   consumes the leading `A` and can then never find `ANCHOR`, so all 29 `determinism.rs` sites
   went to zero. Caught immediately because `--self-test` P1/P2/P3 stopped firing and the main
   check said "no anchor-SHA const site resolved" — i.e. the non-vacuity machinery earned its
   keep on its own author. Fixed to `[A-Za-z0-9_]*ANCHOR[A-Za-z0-9_]*`.
5. **`--pre-commit` only watched `determinism.rs`.** An edit to any of the other four files
   skipped the gate entirely. The pathspec list is now derived from `SCANNED_FILES`, so the two
   cannot drift apart.
6. **`--write` line-number hazard.** `apply_write` rewrites **by line number** and took a single
   path; sites from five files were about to be handed to it. Now grouped per file. It also
   hardcoded `const ANCHOR` / `const ANCHOR_PREFIX` in its substitution, so a `BTC_ANCHOR_SHA`
   site would have no-op'd while still counting a rewrite — it now substitutes the site's own
   const name and **raises** if the substitution does not land.
7. **My own comment nearly became a declaration.** The explanatory note added above the
   `pairs-*` consts originally contained the literal text ``// anchor-ns:`` inside prose. It was
   harmless only because it landed one line outside the 3-line window. Reworded to
   "No namespace declaration here, on purpose". Recorded because it is the same class of
   accident as the bleed in (3): a parser reading prose it was not meant to read.

## 5. The explicit declaration form

`// anchor-scenario: <name>`, read in the same backwards window as `// anchor-ns:` — at most 3
lines, stopping at a `fn` boundary or the previous anchor const — and it **wins** over the
forward adjacency scan.

Four of the six new sites came back UNRESOLVED without it (the tool's correct hard failure, not
a silent drop): two at file scope in `run_yahoo_sma_ticker_flag.rs`, one at file scope in
`reproducibility_sample_figure.rs`, and the Monte-Carlo one whose `const SCENARIO` sits *before*
the digest while the forward scan looks *after* it.

The alternative would have been to plant an incidental quoted string in the test so the regex
finds something. That is how the previous version of this tool laundered one site's meaning into
another (the P3 probe is the memorial). An explicit declaration says what the site pins; an
incidental match guesses.

## 6. Probe results — every probe proved its own mutation first

Two "probes" earlier in this programme were `sed` no-ops reporting OK on unmodified files. So no
step here trusts the edit: every mutation is verified by **re-reading the bytes off disk** and
asserting the new value is present, the old value is absent, and the content differs, before the
tool is run. Scripts: `probe_literals.py`, `probe_floor_and_window.py`,
`verify_write_refusal.py` (session scratchpad, not committed).

### 6.1 One-character literal mutation, per newly-scanned site (6/6 FIRED)

Each probe flipped the first hex character (still hex, so the site still parses — a
non-parsing literal would fail via the floor instead, proving something else).

| file | site | mutation | landed? | tool | drift row names it? |
|---|---|---|---|---|---|
| `multi_pair_determinism.rs` | `pairs-2023-zscore-mr` | `ac647a59…`→`bc647a59…` | yes | exit 1 | yes, `:144` |
| `multi_pair_determinism.rs` | `pairs-2024-h1-zscore-mr` | `5bee5e9c…`→`bbee5e9c…` | yes | exit 1 | yes, `:155` |
| `run_yahoo_sma_ticker_flag.rs` | `btc-yahoo-2024-1d-sma-cross` | `076929bb…`→`b76929bb…` | yes | exit 1 | yes, `:154` |
| `run_yahoo_sma_ticker_flag.rs` | `eth-yahoo-2024-1d-sma-cross` | `c854ff2b…`→`b854ff2b…` | yes | exit 1 | yes, `:170` |
| `theta_surface_reproduction.rs` | `…-real-fy-mc` | `3aae06c0…`→`baae06c0…` | yes | exit 1 | yes, `:362` |
| `reproducibility_sample_figure.rs` | `btc-yahoo-2024-1d-sma-cross` | `076929bb…`→`b76929bb…` | yes | exit 1 | yes, `:49` |

Evidence the mutation landed, per row: `landed=True` from a fresh read (mutated value present,
original absent, bytes differ), a non-empty `git diff --numstat` for the file, **and** the
mutated value appearing in the tool's own drift table — e.g.

```
| eth-yahoo-2024-1d-sma-cross | (file scope) (run_yahoo_sma_ticker_flag.rs:170) | `b854ff2b2a97a876…` | `c854ff2b2a97a876…` | NO |
```

The third of those is the strongest: the tool could only print `b854ff2b…` if it had read the
mutated bytes. Each file was then restored, the restore re-read and confirmed, and the tool
re-run to exit 0 before moving to the next probe.

### 6.2 The non-vacuity floor (2/2 FIRED)

* Floor 35 → **36**: exit 1, `FAIL: found 35 anchor literal site(s), floor is 36`, with the
  per-file breakdown attached.
* Floor 35 → **34**: `--self-test` P4 reports `DID NOT FIRE` and the mode exits 1 — so the floor
  cannot be quietly lowered either. Both mutations verified landed; floor restored to 35 and the
  check re-confirmed green.

### 6.3 The explicit-declaration window (2/2 FIRED)

Removing a single `// anchor-scenario:` line makes the site **UNRESOLVED** — it does not borrow a
neighbour's meaning and it does not pass:

```
run_yahoo_sma_ticker_flag.rs:153 fn (file scope) — BTC_ANCHOR_SHA = 076929bb63d9bec0… (no `// anchor-scenario:` within 3 lines above, and no scenario-shaped string within 7 lines below)
reproducibility_sample_figure.rs:48 fn (file scope) — ANCHORED_BODY_SHA = 076929bb63d9bec0… (no `// anchor-scenario:` within 3 lines above, and no scenario-shaped string within 7 lines below)
```

(Line numbers here are one lower than the § 3 table — the probe deleted a line to create the
condition. Verbatim output, not a discrepancy.)

### 6.4 `--self-test`: 9 probes, all firing

P1 runner spellings · P2 scenario-less const is unresolved · P3 `// anchor-ns:` does not bleed ·
P4 the floor (now ≥ 35) · **P5** non-`ANCHOR`-named consts are seen (`BTC_ANCHOR_SHA`,
`ANCHORED_BODY_SHA`) · **P6** a non-hex value is not an anchor literal (the UI's `"Anchor: "`
label, by name a match and by value not) · **P7** an explicit `// anchor-scenario:` resolves its
own site only · **P8/P8b** `SCANNED_FILES` is non-empty and all present, and a vanished entry
raises instead of counting zero.

### 6.5 `--write` still refuses the non-canonical sites — and how that was verified

`--write` was **never invoked**, per the brief. Verified three ways instead:

1. **The live predicate.** `main()` refuses when `[s for s in sites if s.declared_ns is not None]`
   is non-empty. Evaluated over the real parse: 17 such sites, so the predicate is TRUE and
   `--write` refuses outright. All four new *declared* sites are in that list —
   `BTC_ANCHOR_SHA` (`lab-yahoo-realdata-v0.1.1`), `ETH_ANCHOR_SHA`
   (`lab-yahoo-realdata-v0.1.3`), the MC `ANCHOR` (`mc-robustness-2026-06`) and
   `ANCHORED_BODY_SHA` (`lab-yahoo-realdata-v0.1.1`).
2. **The guard's position, read from the source.** `REFUSING --write` is at line 1047, its
   `return 1` at 1058, and the only `apply_write` call is at 1066 — after it. Checked
   mechanically, not from memory.
3. **`apply_write` exercised on a scratchpad fixture**, never a repo file: it rewrites a
   `BTC_ANCHOR_SHA`-named const correctly, and a substitution that does not land raises
   `RuntimeError` instead of counting a rewrite.

Honest nuance: the two `pairs-*` sites are canonical and therefore **not** individually covered
by the refusal. They are protected only by it being global (any declared site anywhere refuses
the whole run). Their literals already equal their canonical rows, so a write would be a no-op
today — but if `--write` is ever taught to honour `// anchor-ns:`, whoever does that must check
that the canonical resolution for a `v1.5a + …` row is still the unique one.

## 7. Verification

| gate | result |
|---|---|
| `bash scripts/verify_anchors.sh` (before) | `ANCHORS PASS (119 / 119) [declared 119 · floor 119]` |
| `bash scripts/verify_anchors.sh` (after) | `ANCHORS PASS (119 / 119) [declared 119 · floor 119]` |
| `python3 scripts/check_determinism_anchors.py` | exit 0 — 35 of 35 across 5 files |
| `python3 scripts/check_determinism_anchors.py --self-test` | exit 0 — 9 probes fired |
| `rustfmt --edition 2024 --check` on all four edited test files | exit 0, clean |
| `cargo test -p backtest --test multi_pair_determinism` | 4 passed, 0 failed |
| `cargo test -p backtest --features realdata --test theta_surface_reproduction` | 1 passed, 3 ignored (corpus-gated, as declared) |
| `cargo test -p backtest --features yahoo --test run_yahoo_sma_ticker_flag` | 6 passed, 0 failed |
| `cargo test -p backtest --features yahoo --test reproducibility_sample_figure` | 1 passed, 0 failed |

No `evidence/` file and no `_bmad-output/` file was read-modified; no anchor digit anywhere was
changed. Every compile+test step above finished in under 20 s, so no long-running-job watch
block was needed for this work. The corpus-gated θ/MC gates in
`theta_surface_reproduction.rs` are the exception and were **not** run here (13 min cheap /
62 min expensive / 4 min MC, as their `#[ignore]` messages declare); if they are ever run:

```
watch -n 30 'tail -5 /tmp/theta-repro.log; echo; ls -la target/release/monte_carlo 2>/dev/null'
```

## 8. What remains unresolved

Nothing in the brief's scope. One adjacent observation, deliberately **not** acted on:
`reproducibility_sample_figure.rs` carries a whole-file `#![cfg(feature = "yahoo")]`, which the
tool's R3 cfg-gate detection does not recognise (it matches `#[cfg(…)]`, not the inner
`#![cfg(…)]` form). Its literal is therefore reconciled unconditionally, which is the right
outcome — the reconciliation is static and does not depend on whether the test compiles under a
feature — so R3 was left alone rather than quietly widened. Noted so the next reader is not
surprised that a file-gated test's constant is still checked.
