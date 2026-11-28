#!/usr/bin/env python3
# /// script
# requires-python = ">=3.9"
# ///
"""ci_test_target_counts.py — each declared `ui` test target must LIST exactly its declared count.

Story 2-63 (lab recipe test harness v0.3), Close (design § 4.5). Bug-log `#190`,
wiring form.

WHY. A test that a cfg compiles out passes in silence. libtest prints
`running 0 tests`, the binary exits 0, and the CI step stays green. A green
step writes no annotation, and this repository's run logs answer HTTP 403
without admin, so from outside a target with 0 tests and a target with 7 look
the same. Story 2-63 removed gates of exactly that shape:

  * `cfg(unix)` kept tests off the Windows leg in `training_log_state_gating.rs`
    (file level) and `cockpit_training_pressed_wiring.rs` (tests 1-3) until
    Wave E2, and in `activity_tape_training_run.rs` (both tests) until Wave E3;
  * `fixtures` cfg gates kept six files off the macOS leg, whose test step
    passes no `--features fixtures`, until Wave E3 (`#![cfg(feature =
    "fixtures")]` in five of them, six item-level `#[cfg]`s in the sixth);
  * `cockpit_live_lab_run_smoke.rs`'s bug-log `#66` fix reaches CI only
    because `yahoo` is a default ui feature (3 of its 7 tests sit under
    `#[cfg(feature = "yahoo")]`).

The waves recorded the Windows half of each port as UNMEASURED. This gate
turns it into a measurement: it reads `cargo test -p ui ... -- --list` from
the build the leg's test step already made and fails unless every target in
`DECLARED` lists exactly the count declared for that leg.

Measured before it was wired: read with `--format run` against the dev box's
full macOS suite log of 2026-11-21 (the tree before Waves E3 and F), the 6 rows
whose count those waves do not change matched, and the 9 whose count they
change were red at their old counts (2, 0, 0, 0, 0, 2, 0, 3 and 0). The parse
also agreed with that log's own totals: 122 test binaries, 1437 tests.

THE COUNTS ARE DECLARED, NOT MEASURED OFF THE SUBJECT. Each one was read off
the source at the stage that ships it: the `#[test]` / `#[tokio::test]` items
that survive the leg's cfg set. Both legs build ui's default features (`live`,
`yahoo`, `binance`, `live-fx`); Windows adds `fixtures`, which no declared file
has a cfg on. Five declared files are OS-gated at file level (ADR-0057 D2, pixels
are macOS-canonical), so each declares its count on macOS and 0 on Windows:
`activity_tape_failed_hold_render` (bug-log `#217`),
`plan_export_button_render` and `forward_plan_populated_render` (bug-log `#223`
and `#222`) and `lab_custom_range_render` (bug-log `#230`) are
`#![cfg(target_os = "macos")]`, and `lab_stop_render` (bug-log `#215`) is
`#![cfg(all(target_os = "macos", feature = "live"))]`. One more,
`bidi_paragraph_separators` (bug-log `#239`), puts `#[cfg(target_os = "macos")]`
on its 4 pixel items only, so Windows lists 4 fewer. The rows make the macOS-only
AD-10 proofs' counts a measurement instead of a silent zero. No other declared
file puts a `target_os`, `unix` or `windows` cfg on a test item (the `cfg(unix)` /
`cfg(windows)` lines left in two of them gate `use` imports only), so every other
row carries the same number for both legs.

PER LEG, ON PURPOSE. The table still names both legs on every row, with no
default and no inheritance. Each column is a claim of its own (the E2/E3 ports
claimed Windows, the E3 `fixtures` ungating claimed macOS), and a future
OS-gated target must state its count on each leg rather than borrow the other
leg's. `--os` picks the column; the CI step passes it explicitly.

It fails when (violation KINDs, each probed by `--self-test`):

  EMPTY       the input holds no `Running` / `Doc-tests` section at all;
  HEADER      a cargo `Running` line this parser cannot read (cargo's format
              moved; every section after it would be misattributed);
  MISSING     a declared target has no section (renamed, deleted, or never ran);
  DUPLICATE   a declared target has two sections;
  INCOMPLETE  a declared target's section has no count line (the binary died
              mid-listing, or the wrong `--format` was passed);
  INCOHERENT  a declared target's section disagrees with itself (two count
              lines, or a summary that does not match the lines listed);
  COUNT       a declared target lists a different number than declared;
  BAD-ROW     a `DECLARED` row lacks a leg, has a non-integer or negative
              count, or has no reason;
  FLOOR       `DECLARED` holds fewer rows than `MIN_DECLARED`.

Undeclared targets are outside this gate's subject: their sections are parsed
(so the declared ones are attributed correctly) and their problems are printed
as notes, never as violations (bug-log `#189`: a gate that fails for reasons
outside its subject reads as broken).

A changed count is not automatically a defect. A test added to a declared file
means raising its row in the same commit; a test that disappeared means finding
out why before lowering it.

INPUT FORMATS.

  --format list (default)  `cargo test -p ui --tests --no-fail-fast -- --list`:
      cargo prints `     Running tests/<name>.rs (<binary>)` (stderr), then
      libtest prints `<test>: test` per test and `N tests, M benchmarks`
      (`1 test`, singular, for one).
  --format run             the log of a normal `cargo test -p ui` run: the count
      is libtest's `running N tests` line. Use it to check the table against a
      dev-box suite log before a push, without a second exec of every binary.

Both forms are read through ANSI colour codes (CI sets `CARGO_TERM_COLOR:
always`), CRLF line ends, and Windows paths (`tests\\x.rs`, `.exe`).

WIRING. `.github/workflows/ci.yml` runs this after each leg's ui test step,
on the file `scripts/ci_run_annotated.sh` tees to
(`$RUNNER_TEMP/ci-annotated-<label>.log`), so the label in the step and the
path it passes here must match. Not in `.githooks/pre-commit`: it needs a cargo
build of every ui test target, which no commit hook here pays for.

Usage:
    scripts/ci_test_target_counts.py --os {macos,windows} [--format {list,run}] <file|->
    scripts/ci_test_target_counts.py --self-test

Exit status: 0 PASS, 1 a violation (or a self-test probe that did not fire),
2 bad usage or an unreadable input.
"""

from __future__ import annotations

import copy
import os
import re
import sys

LEGS = ("macos", "windows")
FORMATS = ("list", "run")

# ── The declaration ───────────────────────────────────────────────────────────
#
# key = the cargo test target name (`crates/ui/tests/<key>.rs`).
# `macos` / `windows` = the number of tests that target holds on that leg.
# `why` = what the count pins, and how it was derived. Required.
DECLARED: dict[str, dict] = {
    # ── Wave E2 (bug-log #66 fix; training ports off `cfg(unix)`) ────────────
    "cockpit_live_lab_run_smoke": {
        "macos": 7,
        "windows": 7,
        "why": "4 unconditional tests + 3 under `#[cfg(feature = \"yahoo\")]` "
               "(`smoke_yahoo_cache_hit` and its two `should_panic` twins, the "
               "bug-log #66 fix). `yahoo` is a default ui feature on both legs; "
               "without it the count is 4.",
    },
    "training_log_state_gating": {
        "macos": 4,
        "windows": 4,
        "why": "3 lifecycle tests + `stub_child_is_rejected_by_libtest`. The file "
               "was `#![cfg(unix)]` until Wave E2, so Windows listed 0.",
    },
    "cockpit_training_pressed_wiring": {
        "macos": 6,
        "windows": 6,
        "why": "Tests 1-3 were `#[cfg(unix)]` (they spawned `sleep`) until Wave E2; "
               "tests 4-5 were not. Wave E2 added test 6, "
               "`stub_child_is_rejected_by_libtest` (5 -> 6).",
    },
    # ── Wave E3 (Windows port; `fixtures` gates removed; lab_run_engine) ─────
    "activity_tape_training_run": {
        "macos": 3,
        "windows": 3,
        "why": "Both tests were `#[cfg(unix)]` until Wave E3, which dropped the "
               "gates and added `stub_child_is_rejected_by_libtest` (2 -> 3).",
    },
    "audit_filter_chip_emits_filter_changed": {
        "macos": 3,
        "windows": 3,
        "why": "`#![cfg(feature = \"fixtures\")]` until Wave E3: the macOS step "
               "passes no `--features fixtures`, so macOS listed 0.",
    },
    "audit_row_opens_modal": {
        "macos": 2,
        "windows": 2,
        "why": "`#![cfg(feature = \"fixtures\")]` until Wave E3 (macOS listed 0).",
    },
    "home_strategies_row_cross_link": {
        "macos": 3,
        "windows": 3,
        "why": "`#![cfg(feature = \"fixtures\")]` until Wave E3 (macOS listed 0).",
    },
    "inspect_strategy_from_leaderboard": {
        "macos": 6,
        "windows": 6,
        "why": "`#![cfg(feature = \"fixtures\")]` until Wave E3 (macOS listed 0).",
    },
    "open_strategy_in_lab": {
        "macos": 4,
        "windows": 4,
        "why": "`#![cfg(feature = \"fixtures\")]` until Wave E3 (macOS listed 0).",
    },
    "strategies_screen_sparkline_replaces_placeholder": {
        "macos": 1,
        "windows": 1,
        "why": "Six item-level `#[cfg(feature = \"fixtures\")]` gates until Wave E3 "
               "(macOS listed 0).",
    },
    "lab_run_engine": {
        "macos": 2,
        "windows": 2,
        "why": "`inner::h3_in_memory_equals_cached_disk` + (bug-log #212, 1 -> 2) "
               "`inner::every_lab_chip_run_is_read_back_from_its_write_root`, both "
               "under `#[cfg(feature = \"live\")] mod inner`, no OS cfg. Wave E3 "
               "deleted the always-passing `#[cfg(not(feature = \"live\"))]` stub, "
               "so a build without `live` lists 0, not 2.",
    },
    # ── Wave F (the Yahoo preload arm, called off-runtime) ───────────────────
    "spawn_lab_run_yahoo_harness": {
        "macos": 8,
        "windows": 8,
        "why": "6 arm tests + 2 under `#[cfg(feature = \"yahoo\")] mod task_driven` "
               "(6 without `yahoo`). The included `yahoo_arm_support/mod.rs` holds "
               "no test.",
    },
    "lab_runner_ticker_e2e": {
        "macos": 1,
        "windows": 1,
        "why": "`ticker_fires_at_least_3_times_in_1s_window`, rewritten in Wave F "
               "to drive the production arm off-runtime.",
    },
    "lab_runner_cancel_e2e": {
        "macos": 7,
        "windows": 7,
        "why": "The 2 kept names + `stop_during_preload_aborts_preload_task` "
               "(Wave F, 2 -> 3), + tests 4-7 (bug-log #215, 3 -> 7): a preload "
               "Stop and a failed preload landed in the Lab state, an engine "
               "Stop and an engine error through the real `spawn_lab_run` Task. "
               "All four are cross-platform (`live` only).",
    },
    "lab_yahoo_empty_range_classification": {
        "macos": 3,
        "windows": 3,
        "why": "Cases A, B and k2 under `#![cfg(all(feature = \"live\", feature = "
               "\"yahoo\"))]` (Wave F moved the gate from `live` alone).",
    },
    # ── bug-log #217 (the activity tape's purge; the first OS-gated row) ─────
    "activity_tape_failed_hold_render": {
        "macos": 4,
        "windows": 0,
        "why": "T1-T4, the AD-10 pixel proof that a failed activity row is red "
               "inside its 3 s hold and gone after it. `#![cfg(target_os = "
               "\"macos\")]` (ADR-0057 D2), so Windows lists 0 by design; a 0 "
               "on macOS is the silent zero this gate exists to catch.",
    },
    # ── bug-log #215 (a Stop ends Cancelled; the second OS-gated row) ────────
    "lab_stop_render": {
        "macos": 4,
        "windows": 0,
        "why": "R1-R3, the AD-10 pixel proof that a Stop draws \"Cancelled — "
               "Run again\" and no red (with the failed-preload negative control "
               "and the jitter control), + the `#[ignore]` `measure` test, which "
               "`--list` includes. `#![cfg(all(target_os = \"macos\", feature = "
               "\"live\"))]` (ADR-0057 D2), so Windows lists 0 by design.",
    },
    # ── bug-log #231 (layout_invariants: one seed, no separators, recorded labels) ──
    "layout_invariants": {
        "macos": 18,
        "windows": 18,
        "why": "12 layout properties + 6 bug-log #231 tests (separator scan, "
               "free_text two-sided, seed, no persistence, recorded labels laid out "
               "raw, parameter allowlist); the cosmic-text canary was deleted by the "
               "bug-log #239 fix (19 -> 18). No cfg on any item, so both legs list "
               "the same.",
    },
    # ── bug-log #239 (the vendored cosmic-text bidi patch, ADR-0106) ─────────
    "bidi_paragraph_separators": {
        "macos": 19,
        "windows": 15,
        "why": "W1, W2, B1-B3 + 5 parent/child pairs (C1, C2, C3, C3t, C4) on both "
               "legs; + the 2 AD-10 pixel pairs LR-a/LR-b (4 items, item-level "
               "`#[cfg(target_os = \"macos\")]`, ADR-0057 D1), so Windows lists 4 "
               "fewer. The 7 children are `#[ignore]`d (run by their parents in a "
               "child process) and `--list` includes them.",
    },
    # ── bug-log #222 + #223 (the forward plan's provenance header and export gate) ──
    "forward_plan_provenance": {
        "macos": 6,
        "windows": 6,
        "why": "E1-E4 and W1 with no cfg, + E5 under `#[cfg(feature = \"live\")] mod "
               "real_chain` (the real crowned and promoted launch chains). `live` is a "
               "default ui feature on both legs; without it the count is 5, the silent "
               "drop this row catches.",
    },
    "plan_export_button_render": {
        "macos": 3,
        "windows": 0,
        "why": "The 2 Q-HE-6 tests + LR-223 (bug-log #223), the AD-10 pixel proof that "
               "the export button shows only beside the bake-off that crowned the plan. "
               "`#![cfg(target_os = \"macos\")]` (ADR-0057 D2), so Windows lists 0 by "
               "design.",
    },
    "forward_plan_populated_render": {
        "macos": 10,
        "windows": 0,
        "why": "9 tests (populated, negative controls, ADR-0070 Proof 2, RSI, F8, "
               "LR-200b) + LR-222 (bug-log #222), the AD-10 pixel proof that a promoted "
               "plan's header is not the crowned one. `#![cfg(target_os = \"macos\")]` "
               "(ADR-0057 D2), so Windows lists 0 by design.",
    },
    # ── bug-log #230 (the Lab's Custom range editor; ADR-0105) ───────────────
    "lab_custom_range_render": {
        "macos": 11,
        "windows": 0,
        "why": "F1, F2, F2b, F3a, F3b, F4, F5, F6, F7, F8, the AD-10 pixel proof that the "
               "Custom editor colours a bound by the run's own parse, draws an "
               "uncommitted edit, keeps the last run drawn through a partial bound, that "
               "the Yahoo cadence badge reads an RFC 3339 range and that a whole RFC 3339 "
               "bound shows in its field (F8, the review round), + the `#[ignore]` "
               "`measure` test, which `--list` includes. `#![cfg(target_os = "
               "\"macos\")]` (ADR-0057 D2), so Windows lists 0 by design. The "
               "`iced_test::Simulator` tests of the same fix (W2, W5, W5b, K1-K4) live in "
               "the lib, which no row declares, and run on both legs.",
    },
    "lab_custom_range_order": {
        "macos": 3,
        "windows": 3,
        "why": "O1-O3, the real `spawn_lab_run` Task on Synthetic, Yahoo and Binance: a "
               "reversed Custom pair is refused by name before any preload, its valid twin "
               "passes the mapper (bug-log #230 review, ADR-0105 D8). "
               "`#![cfg(feature = \"live\")]`, a default ui feature on both legs, no OS "
               "cfg; without `live` the count is 0, the silent drop this row catches.",
    },
}

# Non-vacuity floor for the declaration itself: deleting rows must be deliberate.
# Raise it when a row is added; lowering it is a decision, not a fix.
MIN_DECLARED = 24

# ── Parsing ───────────────────────────────────────────────────────────────────

ANSI_RE = re.compile(r"\x1b\[[0-9;?]*[ -/]*[@-~]")
# cargo right-aligns its status word in 12 columns: "     Running", "   Doc-tests".
CARGO_RUNNING_RE = re.compile(r"^ {5}Running ")
HEADER_RE = re.compile(r"^ {5}Running (?:unittests )?(?P<src>\S+) \((?P<bin>[^()]+)\)$")
DOCTEST_RE = re.compile(r"^ {3}Doc-tests (?P<crate>\S+)$")
LIST_LINE_RE = re.compile(r"^(?P<name>\S.*): (?P<kind>test|benchmark)$")
LIST_SUMMARY_RE = re.compile(r"^(?P<tests>\d+) tests?, (?P<benches>\d+) benchmarks?$")
RUN_COUNT_RE = re.compile(r"^running (?P<n>\d+) tests?$")

_TEST_SRC_RE = re.compile(r"(?:^|/)tests/(?P<name>[^/]+?)(?:\.rs|/main\.rs)$")
_BIN_SRC_RE = re.compile(r"(?:^|/)src/bin/(?P<name>[^/]+?)(?:\.rs|/main\.rs)$")
_BENCH_SRC_RE = re.compile(r"(?:^|/)benches/(?P<name>[^/]+?)(?:\.rs|/main\.rs)$")
_EXAMPLE_SRC_RE = re.compile(r"(?:^|/)examples/(?P<name>[^/]+?)(?:\.rs|/main\.rs)$")
_HASH_SUFFIX_RE = re.compile(r"-[0-9a-f]{8,}$")


def target_key(src: str, binary: str) -> str:
    """The cargo target a `Running` line names. Integration tests key by bare name."""
    p = src.replace("\\", "/")
    for rx, prefix in (
        (_TEST_SRC_RE, ""),
        (_BIN_SRC_RE, "bin:"),
        (_BENCH_SRC_RE, "bench:"),
        (_EXAMPLE_SRC_RE, "example:"),
    ):
        m = rx.search(p)
        if m:
            return prefix + m.group("name")
    if p == "src/lib.rs" or p.endswith("/src/lib.rs"):
        return "lib"
    if p == "src/main.rs" or p.endswith("/src/main.rs"):
        stem = binary.replace("\\", "/").rsplit("/", 1)[-1]
        if stem.endswith(".exe"):
            stem = stem[: -len(".exe")]
        return "bin:" + _HASH_SUFFIX_RE.sub("", stem)
    return "other:" + p


class Section:
    __slots__ = ("key", "line_no", "list_tests", "list_benches", "summaries", "runs")

    def __init__(self, key: str, line_no: int) -> None:
        self.key = key
        self.line_no = line_no
        self.list_tests = 0
        self.list_benches = 0
        self.summaries: list[tuple[int, int]] = []
        self.runs: list[int] = []


def parse(text: str) -> tuple[list[Section], list[tuple[int, str]]]:
    """Split cargo's output into one section per test binary.

    Returns the sections in order and every `Running` line that did not parse.
    """
    sections: list[Section] = []
    bad_headers: list[tuple[int, str]] = []
    cur: Section | None = None
    for line_no, raw in enumerate(text.splitlines(), 1):
        line = ANSI_RE.sub("", raw).rstrip("\r").rstrip()
        m = HEADER_RE.match(line)
        if m:
            cur = Section(target_key(m.group("src"), m.group("bin")), line_no)
            sections.append(cur)
            continue
        if CARGO_RUNNING_RE.match(line):
            bad_headers.append((line_no, line.strip()))
            cur = None
            continue
        m = DOCTEST_RE.match(line)
        if m:
            cur = Section("doc-tests:" + m.group("crate"), line_no)
            sections.append(cur)
            continue
        if cur is None:
            continue
        body = line.strip()
        m = LIST_LINE_RE.match(body)
        if m:
            if m.group("kind") == "test":
                cur.list_tests += 1
            else:
                cur.list_benches += 1
            continue
        m = LIST_SUMMARY_RE.match(body)
        if m:
            cur.summaries.append((int(m.group("tests")), int(m.group("benches"))))
            continue
        m = RUN_COUNT_RE.match(body)
        if m:
            cur.runs.append(int(m.group("n")))
    return sections, bad_headers


def section_count(s: Section, fmt: str) -> tuple[int | None, tuple[str, str] | None]:
    """(count, None) or (None, (KIND, why)) for one binary's section."""
    if fmt == "list":
        if not s.summaries:
            hint = (
                " — this section holds a `running N tests` line, so the input looks like a "
                "test run's log: pass `--format run`"
                if s.runs else ""
            )
            return None, (
                "INCOMPLETE",
                "printed no `N tests, M benchmarks` line (the binary died before it "
                f"finished listing){hint}",
            )
        if len(s.summaries) > 1:
            return None, (
                "INCOHERENT",
                f"holds {len(s.summaries)} `N tests, M benchmarks` lines — a `Running` "
                "header between two binaries was not recognised",
            )
        tests, benches = s.summaries[0]
        if tests != s.list_tests or benches != s.list_benches:
            return None, (
                "INCOHERENT",
                f"its summary says {tests} test(s) and {benches} benchmark(s), but "
                f"{s.list_tests} `: test` and {s.list_benches} `: benchmark` line(s) "
                "were listed",
            )
        return tests, None
    if not s.runs:
        hint = (
            " — this section holds a `N tests, M benchmarks` line, so the input looks "
            "like `--list` output: drop `--format run`"
            if s.summaries else ""
        )
        return None, (
            "INCOMPLETE",
            f"printed no `running N tests` line (the binary did not start){hint}",
        )
    if len(s.runs) > 1:
        return None, (
            "INCOHERENT",
            f"holds {len(s.runs)} `running N tests` lines — a `Running` header was "
            "not recognised, or a test printed one",
        )
    return s.runs[0], None


def table_violations(table: dict[str, dict[str, object]]) -> list[tuple[str, str, str]]:
    out: list[tuple[str, str, str]] = []
    for key in sorted(table):
        row = table[key]
        if not re.fullmatch(r"[A-Za-z0-9_]+", key):
            out.append(("BAD-ROW", key, "the key is not a cargo test target name"))
        unknown = sorted(set(row) - set(LEGS) - {"why"})
        if unknown:
            out.append(("BAD-ROW", key, f"unknown field(s) {unknown}"))
        for leg in LEGS:
            v = row.get(leg)
            if not isinstance(v, int) or isinstance(v, bool) or v < 0:
                out.append(
                    ("BAD-ROW", key, f"`{leg}` must be a count (int >= 0) for every leg; got {v!r}")
                )
        if not str(row.get("why", "")).strip():
            out.append(("BAD-ROW", key, "no `why`: a declared count with no stated subject"))
    if len(table) < MIN_DECLARED:
        out.append(
            ("FLOOR", "-", f"the table declares {len(table)} target(s); the floor is "
                           f"{MIN_DECLARED}. Rows were deleted, or the floor must move "
                           "deliberately")
        )
    return out


def check(
    text: str,
    leg: str,
    fmt: str,
    table: dict[str, dict[str, object]] | None = None,
) -> tuple[list[tuple[str, str, str]], list[str], dict[str, object]]:
    """Return (violations as (KIND, target, message), notes, stats)."""
    table = DECLARED if table is None else table
    violations = table_violations(table)
    notes: list[str] = []
    sections, bad_headers = parse(text)
    stats: dict[str, object] = {"sections": len(sections), "matched": {}}
    for line_no, line in bad_headers:
        violations.append(
            ("HEADER", "-", f"line {line_no}: cargo `Running` line not understood: {line!r}")
        )
    if not sections:
        violations.append(
            ("EMPTY", "-", "no `Running` or `Doc-tests` section in the input: it is empty, "
                           "truncated before the first binary, or not cargo test output")
        )
        return violations, notes, stats

    by_key: dict[str, list[Section]] = {}
    for s in sections:
        by_key.setdefault(s.key, []).append(s)
    total = 0
    for s in sections:
        n, _ = section_count(s, fmt)
        total += n or 0
    stats["tests"] = total

    matched: dict[str, int] = {}
    for target in sorted(table):
        want = table[target].get(leg)
        found = by_key.get(target, [])
        if not found:
            violations.append(
                ("MISSING", target, f"no `Running tests/{target}.rs` section: the target "
                                    "was renamed or deleted, or its binary never ran")
            )
            continue
        if len(found) > 1:
            lines = ", ".join(str(s.line_no) for s in found)
            violations.append(
                ("DUPLICATE", target, f"{len(found)} sections (lines {lines}); the input "
                                      "concatenates two runs, or two packages were tested")
            )
            continue
        n, problem = section_count(found[0], fmt)
        if problem is not None:
            violations.append((problem[0], target, f"section at line {found[0].line_no} {problem[1]}"))
            continue
        if n != want:
            violations.append(
                ("COUNT", target, f"declared {want} on {leg}, listed {n}. A cfg compiled "
                                  "tests out, or tests were added or removed without "
                                  "updating DECLARED")
            )
            continue
        matched[target] = n
    stats["matched"] = matched

    for key in sorted(by_key):
        if key in table:
            continue
        secs = by_key[key]
        if len(secs) > 1:
            notes.append(f"undeclared {key}: {len(secs)} sections")
        for s in secs:
            _, problem = section_count(s, fmt)
            if problem is not None:
                notes.append(f"undeclared {key} (line {s.line_no}): {problem[0]}: {problem[1]}")
    return violations, notes, stats


# ── Output ────────────────────────────────────────────────────────────────────


def _gh_escape(s: str) -> str:
    return s.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")


def annotate(title: str, lines: list[str], budget: int = 9) -> None:
    """GitHub ANNOTATIONS (readable without admin; logs are 403). ~10 per step."""
    if os.environ.get("GITHUB_ACTIONS") != "true":
        return
    for line in lines[:budget]:
        print(f"::error title={_gh_escape(title)}::{_gh_escape(line)}")
    if len(lines) > budget:
        print(
            f"::error title={_gh_escape(title)}::annotation budget reached — "
            f"{len(lines)} violation(s) in total; see the step log for the rest"
        )


def run_check(path: str, leg: str, fmt: str) -> int:
    try:
        if path == "-":
            text = sys.stdin.read()
        else:
            with open(path, encoding="utf-8", errors="replace") as fh:
                text = fh.read()
    except OSError as e:
        print(f"ci-test-target-counts: cannot read {path!r}: {e}", file=sys.stderr)
        annotate(f"ui test-target counts ({leg})", [f"cannot read {path!r}: {e}"])
        return 2

    violations, notes, stats = check(text, leg, fmt)
    matched = stats.get("matched", {})
    print(
        f"ui test-target counts: leg {leg}, format {fmt}, input {path}: "
        f"{stats.get('sections', 0)} test binaries, {stats.get('tests', 0)} tests"
    )
    for target in sorted(DECLARED):
        if target in matched:
            print(f"  OK        {target:52} {matched[target]}")
    for kind, target, msg in violations:
        print(f"  {kind:9} {target:52} {msg}")
    for note in notes:
        print(f"  note      {note}")
    if violations:
        print(
            f"ci-test-target-counts: FAIL ({len(violations)} violation(s); "
            f"{len(matched)}/{len(DECLARED)} declared targets match on {leg})"
        )
        annotate(
            f"ui test-target counts ({leg})",
            [f"{kind} {target}: {msg}" for kind, target, msg in violations],
        )
        return 1
    print(
        f"ci-test-target-counts: PASS ({len(matched)}/{len(DECLARED)} declared targets "
        f"list exactly their declared count on {leg})"
    )
    return 0


# ── Self-test ─────────────────────────────────────────────────────────────────

ESC = "\x1b"


def _render(
    sections: list[dict[str, object]],
    *,
    fmt: str,
    windows: bool,
    ansi: bool = True,
) -> str:
    """Cargo-shaped output for synthetic binaries.

    Each section: {"src": "tests/x.rs" | "src/lib.rs" | ..., "n": int,
    optional "summary": False (drop the count line), "skew": int (summary
    claims n but lists n - skew lines), "header": str (a raw replacement line)}.
    """
    out: list[str] = [
        "   Compiling ui v0.1.0 (/w/crates/ui)",
        "    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.00s",
    ]
    status = (f"{ESC}[1m{ESC}[92m     Running{ESC}[0m" if ansi else "     Running")
    for i, sec in enumerate(sections):
        src = str(sec["src"])
        n = int(sec["n"])
        stem = src.rsplit("/", 1)[-1].rsplit(".", 1)[0]
        if src.endswith("src/lib.rs"):
            stem = "ui"
        binary = f"target/debug/deps/{stem}-{i:016x}"
        if windows:
            src = src.replace("/", "\\")
            binary = binary.replace("/", "\\") + ".exe"
        unit = "unittests " if src.replace("\\", "/").startswith("src/") else ""
        out.append(str(sec.get("header", f"{status} {unit}{src} ({binary})")))
        if fmt == "list":
            listed = n - int(sec.get("skew", 0))
            for t in range(listed):
                out.append(f"{stem}::case_{t}: test")
            if sec.get("summary", True):
                if n:
                    out.append("")
                out.append(f"{n} {'test' if n == 1 else 'tests'}, 0 benchmarks")
        else:
            out.append("")
            if sec.get("summary", True):
                out.append(f"running {n} {'test' if n == 1 else 'tests'}")
                for t in range(n):
                    out.append(f"test {stem}::case_{t} ... ok")
                out.append("")
                out.append(
                    f"test result: ok. {n} passed; 0 failed; 0 ignored; 0 measured; "
                    "0 filtered out; finished in 0.01s"
                )
            out.append("")
    sep = "\r\n" if windows else "\n"
    return sep.join(out) + sep


def _sections_for(
    leg: str, table: dict[str, dict[str, object]], *, windows: bool
) -> list[dict[str, object]]:
    """The declared targets at their declared counts, among undeclared fillers."""
    secs: list[dict[str, object]] = [
        {"src": "src/lib.rs", "n": 12},
        {"src": "src/bin/cockpit_live.rs", "n": 0},
        {"src": "src/bin/viewer.rs", "n": 2},
    ]
    if windows:
        secs.append({"src": "src/bin/ui-gallery.rs", "n": 0})
    secs.append({"src": "tests/_undeclared_visual.rs", "n": 0 if windows else 5})
    for name in sorted(table):
        secs.append({"src": f"tests/{name}.rs", "n": int(table[name][leg])})
    secs.append({"src": "tests/zz_undeclared_tail.rs", "n": 1})
    return secs


def _replace(secs: list[dict[str, object]], name: str, **kw: object) -> list[dict[str, object]]:
    out = copy.deepcopy(secs)
    for s in out:
        if s["src"] == f"tests/{name}.rs":
            s.update(kw)
            return out
    raise AssertionError(f"self-test fixture has no section for {name}")


def self_test() -> int:
    """Probe the gate. Each perturbation must raise the violation KIND it targets.

    A probe passes only when the expected KIND fires for the expected target and
    the probe's input differs from its control, so an inert probe cannot read OK.
    Every control (macOS list, Windows list, macOS run log) must pass with zero
    violations, which is the other half of each pair.
    """
    print("ci-test-target-counts --self-test")
    rc = 0

    def ok(msg: str) -> None:
        print(f"  OK    {msg}")

    def fail(msg: str) -> None:
        nonlocal rc
        rc = 1
        print(f"  FAIL  {msg}")

    def fires(viol: list[tuple[str, str, str]], kind: str, target: str | None) -> bool:
        return any(k == kind and (target is None or t == target) for k, t, _ in viol)

    # ── The declaration itself ────────────────────────────────────────────────
    own = table_violations(DECLARED)
    if own:
        fail(f"the live DECLARED table has {len(own)} violation(s): {own[0]}")
    else:
        ok(f"control: the live DECLARED table is well-formed ({len(DECLARED)} rows, "
           f"floor {MIN_DECLARED})")

    # ── Controls: every leg/format pair passes clean ──────────────────────────
    controls = {
        ("macos", "list"): _render(_sections_for("macos", DECLARED, windows=False),
                                   fmt="list", windows=False),
        ("windows", "list"): _render(_sections_for("windows", DECLARED, windows=True),
                                     fmt="list", windows=True),
        ("macos", "run"): _render(_sections_for("macos", DECLARED, windows=False),
                                  fmt="run", windows=False),
    }
    # The fixtures must actually carry the quirks they claim to exercise.
    quirks = {
        "ANSI colour on the `Running` lines": all(f"{ESC}[" in t for t in controls.values()),
        "CRLF and backslash paths and `.exe` on Windows":
            "\r\n" in controls[("windows", "list")]
            and "tests\\" in controls[("windows", "list")]
            and ".exe)" in controls[("windows", "list")],
        "a singular `1 test, 0 benchmarks` summary": "\n1 test, 0 benchmarks" in
            controls[("macos", "list")].replace("\r\n", "\n"),
        "a singular `running 1 test` line": "\nrunning 1 test\n" in controls[("macos", "run")],
    }
    for label, present in quirks.items():
        if present:
            ok(f"fixture carries {label}")
        else:
            fail(f"fixture is missing {label}: the controls below would not exercise it")
    for (leg, fmt), text in controls.items():
        viol, _, stats = check(text, leg, fmt)
        if viol:
            fail(f"control {leg}/{fmt} raised {len(viol)} violation(s), first {viol[0]}")
        else:
            ok(f"control {leg}/{fmt}: PASS, {len(stats['matched'])} declared targets "
               f"matched among {stats['sections']} sections")

    # ── Per-target sweeps: wrong count (both directions) and a missing target ─
    for leg, windows in (("macos", False), ("windows", True)):
        base = _sections_for(leg, DECLARED, windows=windows)
        control = _render(base, fmt="list", windows=windows)
        for label, kind, mutate in (
            ("listed one test short", "COUNT",
             lambda n: _replace(base, n, n=int(DECLARED[n][leg]) - 1)),
            ("listed one test over", "COUNT",
             lambda n: _replace(base, n, n=int(DECLARED[n][leg]) + 1)),
            ("absent from the listing", "MISSING",
             lambda n: [s for s in base if s["src"] != f"tests/{n}.rs"]),
        ):
            missed = []
            tried = 0
            for name in sorted(DECLARED):
                if kind == "COUNT" and label.endswith("short") and int(DECLARED[name][leg]) == 0:
                    continue
                tried += 1
                text = _render(mutate(name), fmt="list", windows=windows)
                if text == control:
                    missed.append(f"{name} (inert: the probe changed nothing)")
                    continue
                viol, _, _ = check(text, leg, "list")
                if not fires(viol, kind, name):
                    missed.append(name)
            if missed:
                fail(f"{kind:10} did NOT fire on {leg} for: {', '.join(missed)}")
            else:
                ok(f"{kind:10} fires on {leg} for each of the {tried} declared targets {label}")

    # ── Single probes ─────────────────────────────────────────────────────────
    mac = _sections_for("macos", DECLARED, windows=False)
    mac_list = controls[("macos", "list")]
    first = sorted(DECLARED)[0]
    multi = next(n for n in sorted(DECLARED) if int(DECLARED[n]["macos"]) >= 2)
    singles: list[tuple[str, str, str | None, str, str, str, dict[str, dict[str, object]] | None]] = [
        ("an empty listing", "EMPTY", None, "", "macos", "list", None),
        ("cargo compile noise and no test binary", "EMPTY", None,
         "   Compiling ui v0.1.0 (/w/crates/ui)\n"
         "    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.00s\n",
         "macos", "list", None),
        (f"{multi}'s binary died before its summary", "INCOMPLETE", multi,
         _render(_replace(mac, multi, summary=False), fmt="list", windows=False),
         "macos", "list", None),
        (f"{multi}'s summary disagrees with its listed lines", "INCOHERENT", multi,
         _render(_replace(mac, multi, skew=1), fmt="list", windows=False),
         "macos", "list", None),
        (f"{first} listed twice", "DUPLICATE", first,
         _render(mac + [s for s in mac if s["src"] == f"tests/{first}.rs"],
                 fmt="list", windows=False),
         "macos", "list", None),
        ("a `Running` line without its `(binary)`", "HEADER", None,
         _render(_replace(mac, first, header=f"     Running tests/{first}.rs"),
                 fmt="list", windows=False),
         "macos", "list", None),
        ("a test run's log read as `--list`", "INCOMPLETE", first,
         controls[("macos", "run")], "macos", "list", None),
        ("`--list` output read as a test run's log", "INCOMPLETE", first,
         mac_list, "macos", "run", None),
        (f"a run log where {multi} ran one test short", "COUNT", multi,
         _render(_replace(mac, multi, n=int(DECLARED[multi]["macos"]) - 1),
                 fmt="run", windows=False),
         "macos", "run", None),
    ]

    # The OS dimension, as a two-sided pair: ONE listing, a table whose two
    # columns differ on one row, read once per leg. Only the column changes, so
    # only the column can make the outcomes differ.
    skewed = copy.deepcopy(DECLARED)
    skewed[first]["windows"] = int(skewed[first]["macos"]) + 2
    singles.append(
        ("the windows leg reading a table whose windows column differs", "COUNT", first,
         mac_list.replace("\r\n", "\n"), "windows", "list", skewed)
    )

    bad_leg = copy.deepcopy(DECLARED)
    del bad_leg[first]["windows"]
    singles.append(("a row with no windows count", "BAD-ROW", first, mac_list, "macos", "list", bad_leg))
    bad_neg = copy.deepcopy(DECLARED)
    bad_neg[first]["macos"] = -1
    singles.append(("a negative count", "BAD-ROW", first, mac_list, "macos", "list", bad_neg))
    bad_why = copy.deepcopy(DECLARED)
    bad_why[first]["why"] = "  "
    singles.append(("a row with no reason", "BAD-ROW", first, mac_list, "macos", "list", bad_why))
    shrunk = {k: DECLARED[k] for k in sorted(DECLARED)[: MIN_DECLARED - 1]}
    singles.append(("a table below the floor", "FLOOR", None, _render(
        _sections_for("macos", shrunk, windows=False), fmt="list", windows=False),
        "macos", "list", shrunk))

    for label, kind, target, text, leg, fmt, table in singles:
        viol, _, _ = check(text, leg, fmt, table)
        if fires(viol, kind, target):
            ok(f"{kind:10} fires on: {label}")
        elif viol:
            fail(f"{kind:10} did NOT fire on: {label}; got {viol[0][0]} instead, so the "
                 "branch under test was never reached")
        else:
            fail(f"{kind:10} INERT on: {label}; no violation at all")

    # The other side of the OS pair: the same listing and table on the macOS leg.
    viol, _, _ = check(mac_list, "macos", "list", skewed)
    if viol:
        fail(f"OS pair: the macOS leg should pass on the skewed table; got {viol[0]}")
    else:
        ok("OS pair: the same listing and table PASS on macos (only the column differs)")

    # Undeclared targets are outside the subject: their problems are notes.
    noisy = _render(
        [dict(s, summary=False) if s["src"] == "tests/zz_undeclared_tail.rs" else s for s in mac],
        fmt="list", windows=False,
    )
    viol, notes, _ = check(noisy, "macos", "list")
    if viol:
        fail(f"an undeclared binary's broken listing raised a violation: {viol[0]}")
    elif not notes:
        fail("an undeclared binary's broken listing raised no note either (silent)")
    else:
        ok("an undeclared binary's broken listing is a note, not a violation")

    print(
        "ci-test-target-counts: SELF-TEST PASS"
        if rc == 0
        else "ci-test-target-counts: SELF-TEST FAIL"
    )
    if rc:
        annotate("ui test-target counts self-test", ["a probe did not fire; see the step log"])
    return rc


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        return self_test()
    leg = fmt = path = None
    fmt = "list"
    it = iter(argv)
    for arg in it:
        if arg == "--os":
            leg = next(it, None)
        elif arg == "--format":
            fmt = next(it, None)
        elif arg in ("-h", "--help"):
            print(__doc__)
            return 0
        elif path is None and (arg == "-" or not arg.startswith("-")):
            path = arg
        else:
            print(f"ci-test-target-counts: unexpected argument {arg!r}", file=sys.stderr)
            return 2
    if leg not in LEGS or fmt not in FORMATS or path is None:
        print(
            "usage: ci_test_target_counts.py --os {macos,windows} [--format {list,run}] <file|->\n"
            "       ci_test_target_counts.py --self-test",
            file=sys.stderr,
        )
        return 2
    return run_check(path, leg, fmt)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
