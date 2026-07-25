---
slug: advisor-narration-faithfulness
status: shipped
owner: operator
version: 2.0.0
updated: 2026-07-01
---

# P2-1 Narration Faithfulness Hardening

Tightens the F9 LLM narration post-check (ADR-0064) so the narrator can
never predict, cause, or invent. An **additive amendment** to the shipped
faithfulness guard — the existing P1/P2/P3/P4 predicate shape is unchanged;
this ships two hardening layers on top.

**Design reference:** [`v2-architecture.md`](../v2-architecture.md) §1
P2-1. Research:
[`research/llms/application-llm-narration-and-agents.md`](../../../research/llms/application-llm-narration-and-agents.md)
§6 P0 — LLM narration hallucination is the one real risk on the shipped F9
seam. ADR: [`0064-advisor-llm-narration-seam.md`](../../../_bmad-output/planning-artifacts/architecture/decisions/0064-advisor-llm-narration-seam.md)
§ "Amendment 2026-07-01 (P2-1 faithfulness hardening)" — D9/D10/D11.

## What shipped

1. **Verbatim-number match, hardened representation (D9).**
   `NarrationFacts::allowed_numbers()` now returns an owned
   `HashSet<String>` (was `Vec<String>`, converted to a `HashSet` inline at
   the `check_faithful` call site on every invocation). The match
   discipline itself is UNCHANGED — exact-string, never float-tolerant
   (ADR-0064 § D2's original design) — this closes a representational gap
   and fixes the audit trail: `RejectReason::FabricatedNumber(String)` and
   `RejectReason::BannedPhrase(String)` now carry the offending
   token/phrase (were unit variants). `WrongCrown` and
   `ContradictedOutcome` are unchanged; `FaithfulnessVerdict::Pass` is
   semantically identical to before this change.

2. **Extended banned-phrase list (D10).** `BANNED_PHRASES` (ADR-0064 §
   D2.P4) gains three additive categories on top of the frozen 42-phrase
   list (no phrase removed, no existing phrase's match altered):
   - **Prediction verbs:** `expected to`, `forecast`, `predict`,
     `probably`, `likely to`, `anticipates`, `projected`.
   - **Causation clauses:** `because of`, `driven by`, `caused by`,
     `due to`.
   - **Advice/recommendation phrases:** `you should`, `we recommend`,
     `invest in`, `stay away from`.
   (`will rise`/`will fall`/`buy now`/`sell now` were already in the
   original list — not duplicated.) Match is case-insensitive substring,
   scanned in array order, first hit wins — same discipline as before.

3. **27-test adversarial corpus (D11).** New
   [`crates/agent/tests/narration_faithfulness.rs`](../../../crates/agent/tests/narration_faithfulness.rs)
   through the crate's public API: 1 positive, 3 number-invention, 9
   prediction, 4 causation, 8 recommendation, 2 backward-compatibility
   proofs (a pre-P2-1 faithful narration for both `ActiveWins` and
   `AllFragile` outcomes still passes). All 27 pass. The pre-existing 25
   unit tests in `narration.rs` were updated (only their `assert_eq!`
   literals gained the new `RejectReason` payload argument — no test's
   expected outcome changed) and still pass.

## Location (binding)

`crates/agent/src/narration.rs` (the existing F9 module) + new
`crates/agent/tests/narration_faithfulness.rs`. No new crate, no new
dependency edge — `crates/llm` and `crates/backtest` were already hard
deps of `agent`; `strategy`/`exec`/`models`/`ui` untouched.

## Anchor safety

Unchanged from the original ADR-0064 § D7 — narration is display-only and
ephemeral, produces no anchored artifact. `scripts/verify_anchors.sh`
119/119 both before and after this change (verified).

## Changelog

- 2026-07-01 (developer): shipped. Verbatim-number-match hardening +
  extended banned-phrase list + 27-test adversarial corpus. ADR-0064
  amended (§ "Amendment 2026-07-01"). Verified: `cargo test -p agent --lib`
  101/101, `cargo test -p agent --test narration_faithfulness` 27/27,
  `cargo test -p llm --lib` 108/108, `cargo clippy -p agent -p llm --tests
  -- -D warnings` clean, `cargo fmt --check` clean, `verify_anchors.sh`
  119/119 (before + after).
