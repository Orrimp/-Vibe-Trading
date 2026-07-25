# ui-debugger — charter note (no v6 twin)

> Written during BMAD-migration Phase 5a
> (docs/dev-notes/bmad-migration-plan-2026-07-24.md § 6). This is a
> **preservation note, not a functional override** — there is no
> `bmad-agent-ui-debugger` skill directory for `resolve_customization.py`
> to merge this against, so nothing reads this file automatically. It
> exists so the charter survives Phase 5c's `.claude/agents/*.md`
> retirement even though the migration plan has no clean BMAD-native slot
> for it today.
>
> **Disposition until Phase 5c:** unchanged. The agent keeps running
> exactly as `.claude/agents/ui-debugger.md` describes, invoked by name
> for UI bugs. Its render-verification ladder is ALSO already folded live
> into `_bmad/custom/bmad-agent-ux-designer.toml` and
> `_bmad/custom/bmad-agent-dev.toml` (both cite
> `docs/dev-notes/iced-ui-render-verification.md` in `persistent_facts`),
> so the core discipline is not orphaned even before this charter's own
> re-instantiation — this note captures the DEBUGGING-SPECIFIC procedure
> that belongs to neither of those two implementation-facing personas.

## Why no v6 twin

Route a UI **bug** ("no graph", "blank panel", "looks wrong", a mystery
test failure) here; route UI **design/implementation** to
`bmad-agent-ux-designer`. BMAD has nothing resembling a rendered-pixel
debugging specialist with a tool-selection ladder — this is bespoke to
this project's iced 0.14 + tiny-skia stack.

## Condensed charter (full source: `.claude/agents/ui-debugger.md`)

- **The cardinal rule:** verify at the rendered-PIXEL layer, exercising the
  POPULATED/non-trivial state, with a negative control — and actually
  `Read` the PNG. Unit tests, text summaries, and a no-panic boot are
  proxies that go green while the screen is blank.
- **Six-tool ladder, cheapest/most-reproducible first:**
  1. Headless render (`iced_test::screenshot`/`Emulator::screenshot`) — YOU
     run it; the only acceptable proof a thing draws.
  2. Test recorder/simulator (`iced_test::simulator` + `tester` feature) —
     YOU run it; for click/selection/sequence bugs.
  3. comet (live inspector, `debug` feature + iced_beacon, F12) —
     ORCHESTRATOR/OPERATOR only (needs a live window).
  4. Time-travel (rewind message/state history) — ORCHESTRATOR/OPERATOR
     only; needs `application::timed` wiring the cockpit doesn't have yet.
  5. Hot-reload (`cargo-hot`/`chaud`) — ORCHESTRATOR/OPERATOR; fast
     visual-iteration accelerator, not a correctness gate.
  6. Tracing/`iced_debug` spans — YOU run it; cheapest first probe.
- **The debug loop:** reproduce headless RED first -> `Read` the PNG and
  state concretely what you see -> localize (empty-by-data is not a bug;
  feature-path divergence between `cargo test` and the live binary is the
  most common root cause; state-never-reaches-widget; a fixture that only
  ever rendered the trivial state) -> fix the smallest correct change ->
  re-prove with pixels and leave a durable positive+negative-control test
  -> run the full gate (`cargo test -p ui`, a forced clippy re-lint,
  `cargo fmt --check`, `verify_anchors.sh` staying 119/119).
- **Constraints:** files only, no git; never edit anchored reports or touch
  `product.md`/ADRs unless the bug is genuinely there; revert any temporary
  diagnostic feature flags before handing back; independently re-`Read`
  any PNG you cite, never claim a pixel count you didn't verify.

## What a future re-instantiation would need to preserve

1. The tool-selection ladder itself (which of the six tools for which
   symptom) — this is the single highest-value piece of institutional
   knowledge in the source file and is NOT self-evident from iced's own
   docs.
2. The "empty-by-data is not a bug, confirm this FIRST" localization step —
   the single most common false-positive this agent exists to prevent.
3. The orchestrator/operator hand-off protocol for the three live-window
   tools (comet/time-travel/hot-reload) — a sub-agent claiming to have run
   these without a real window would be lying.

Re-instantiation path, operator to decide when this becomes live work: a
project-custom `bmad-ui-debug` workflow skill (via `bmad-builder`) that
`bmad-agent-ux-designer` or `bmad-agent-dev` dispatches to by menu code,
carrying the ladder as its step file. Not built yet — this file is scoped
to Phase 5a only.
