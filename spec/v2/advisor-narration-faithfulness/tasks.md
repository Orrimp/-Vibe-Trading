---
slug: advisor-narration-faithfulness
status: dev-done
owner: developer
updated: 2026-07-01
---

# Tasks — P2-1 Narration Faithfulness Hardening

## Completed by developer (2026-07-01)

- [x] T1 — Harden `NarrationFacts::allowed_numbers()` to return an owned
  `HashSet<String>` (verbatim-number-match representation).
  - file: `crates/agent/src/narration.rs:200-227` (`allowed_numbers` fn body)
  - test: `cargo test -p agent --lib narration::tests::allowed_numbers_includes_sortino_and_calmar`
  - output: `test narration::tests::allowed_numbers_includes_sortino_and_calmar ... ok`

- [x] T2 — Extend `RejectReason::FabricatedNumber` and `::BannedPhrase` to
  carry the offending token/phrase (were unit variants).
  - file: `crates/agent/src/narration.rs:304-328` (`RejectReason` enum),
    `:580-583` (P4 construction site), `:711-718` (P3 construction site)
  - test: `cargo test -p agent --lib narration`
  - output: `test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 76 filtered out; finished in 0.00s`

- [x] T3 — Extend `BANNED_PHRASES` with prediction verbs, causation
  clauses, and advice/recommendation phrases (P2-1 additive categories).
  - file: `crates/agent/src/narration.rs:330-397` (`BANNED_PHRASES` const)
  - test: `cargo test -p agent --test narration_faithfulness prediction_ causation_ recommendation_`
  - output: `test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`
    (21 of the 27 are prediction/causation/recommendation tests, run as
    part of the full 27-test suite above)

- [x] T4 — Update the 6 pre-existing unit tests whose `assert_eq!`
  literals needed the new `RejectReason` payload argument (no test's
  expected outcome changed).
  - file: `crates/agent/src/narration.rs:1378-1454` (`d4_p3_fabricated_number_rejects`,
    `d4_p4_banned_phrase_will_rise_rejects`, `d4_p4_banned_phrase_expected_return_rejects`,
    `d4_p4_guaranteed_rejects`, `d4_p4_you_should_buy_rejects`),
    `:1644` (`unfaithful_sortino_still_rejects`)
  - test: `cargo test -p agent --lib narration`
  - output: `test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 76 filtered out; finished in 0.00s`

- [x] T5 — Write the 27-test adversarial corpus (1 positive, 3
  number-invention, 9 prediction, 4 causation, 8 recommendation, 2
  backward-compat) as a new integration test.
  - file: `crates/agent/tests/narration_faithfulness.rs:1` (new file, 27 tests)
  - test: `cargo test -p agent --test narration_faithfulness`
  - output: `test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`

- [x] T6 — Fix a clippy `unnecessary_to_owned` hit surfaced by the
  `HashSet<String>` type change in a pre-existing test
  (`allowed_numbers_includes_sortino_and_calmar`).
  - file: `crates/agent/src/narration.rs:1580-1594`
  - test: `cargo clippy -p agent -p llm --tests -- -D warnings`
  - output: `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 0.83s` (exit 0, zero warnings)

- [x] T7 — Amend `spec/architecture/adr/0064-advisor-llm-narration-seam.md`
  with the `## Amendment 2026-07-01 (P2-1 faithfulness hardening)` section
  (D9/D10/D11) + a Changelog entry; bump the frontmatter `date:`.
  - file: `spec/architecture/adr/0064-advisor-llm-narration-seam.md:5`
    (date bump), `:436-540` (Amendment section), `:544-568` (Changelog entry)
  - test: `python3 scripts/adr_registry_check.py --self-test`
  - output: `Ran 5 tests in 0.004s\n\nOK`

- [x] T8 — Update the ADR registry row for ADR-0064 in
  `spec/architecture/adr/README.md` § Registry to note the amendment
  (keeps the index-row honest per the amendment).
  - file: `spec/architecture/adr/README.md:114`
  - test: `python3 scripts/adr_registry_check.py --self-test`
  - output: `Ran 5 tests in 0.004s\n\nOK`

- [x] T9 — Create `spec/v2/advisor-narration-faithfulness/feature.md` +
  `tasks.md` (this file).
  - file: `spec/v2/advisor-narration-faithfulness/feature.md`
  - file: `spec/v2/advisor-narration-faithfulness/tasks.md` (this file)
  - test: `python3 scripts/spec_lint.py`
  - output: (see verification section below)

- [x] T10 — Add `REQ-V2-P2-1-NARRATION-FAITHFULNESS-001` row to
  `spec/trace.toml`.
  - file: `spec/trace.toml` (new `[[req]]` block appended)
  - test: `python3 scripts/spec_lint.py`
  - output: (see verification section below)

## Verification summary (developer, 2026-07-01)

- `cargo test -p agent --lib` — `test result: ok. 101 passed; 0 failed;
  0 ignored; 0 measured; 0 filtered out; finished in 56.99s`
- `cargo test -p agent --test narration_faithfulness` — `test result: ok.
  27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in
  0.00s`
- `cargo test -p llm --lib` — `test result: ok. 108 passed; 0 failed; 1
  ignored; 0 measured; 0 filtered out; finished in 0.02s` (the 1 ignored
  test is pre-existing, unrelated to this change)
- `cargo clippy -p agent -p llm --tests -- -D warnings` — clean, exit 0
- `cargo fmt --check` — clean, exit 0 (after one `cargo fmt` auto-fix pass
  on the new test file's manual line-wrapping)
- `bash scripts/verify_anchors.sh` — `ANCHORS PASS (119 / 119)`, run
  before AND after the code change
- `python3 scripts/adr_registry_check.py --self-test` — `Ran 5 tests in
  0.004s\n\nOK`

## For the tester to verify

- [ ] T_FINAL_1 — Re-run the full gate list above independently.
- [ ] T_FINAL_2 — Confirm `python3 scripts/spec_lint.py` PASS with the new
  `feature.md`/`tasks.md`/trace.toml row in place.
- [ ] T_FINAL_3 — Confirm no external crate (outside `agent`'s own test
  module and the new `narration_faithfulness.rs`) matches on
  `RejectReason::FabricatedNumber`/`::BannedPhrase` in a way broken by the
  new payload (grep-confirmed clean by the developer; tester to
  independently re-verify).
- [ ] T_FINAL_4 — Confirm the FROZEN gate identity proofs stay green (no
  rank-path touch expected; narration is display-only).
