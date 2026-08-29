#!/usr/bin/env bash
# Run a test command and turn its failures into GitHub Actions ANNOTATIONS.
#
# WHY THIS EXISTS
#
# Downloading a run's logs requires repo ADMIN (`gh run view --log-failed` ->
# HTTP 403 for anyone else, confirmed against api.github.com directly). A
# non-admin therefore sees a red run as nothing more than the word "failure"
# beside a step name. Diagnosing the 2026-08-22 breakage cost a full clean-clone
# reproduction of the entire workspace to recover a fact that one line of the log
# already held: which test failed.
#
# `::error::` lines become run ANNOTATIONS, which live in the run's METADATA
# rather than its log, and are readable without admin:
#
#   gh run view <id>                                     # ANNOTATIONS section
#   gh api repos/<o>/<r>/check-runs/<id>/annotations --hostname github.com
#
# TWO TRAPS THIS SCRIPT EXISTS TO AVOID, both paid for once already:
#
#  1. `shell: bash` in a workflow runs `bash --noprofile --norc -eo pipefail`.
#     `-e` is ALREADY ON, so a failing pipeline aborts the script instantly and
#     every diagnostic line after it is unreachable. The first version of this
#     logic emitted ZERO annotations for exactly that reason. Hence `set +e`
#     around the command, and `${PIPESTATUS[0]}` to recover the real status
#     through the `tee`.
#  2. A BUILD or LINK failure produces no `test ... FAILED` line at all, so
#     matching only on test names would annotate nothing on the very failures
#     that are hardest to diagnose remotely. `error:` / `error[E....]` lines are
#     matched too.
#
#  3. GitHub keeps at most ~10 annotations PER STEP. The first version emitted
#     every failing test NAME first, which exhausted that budget before a single
#     `panicked at` line — the lines that actually carry file:line AND the
#     assertion text. On the run for bf6ee74 the windows workspace step had ~10
#     failures and surfaced ZERO panics. Ordering is therefore load-bearing:
#     SUMMARY first (so the true count always survives), then panics WITH their
#     message line, then names, then an explicit elision notice. Never let the
#     cheap lines crowd out the informative ones.
#  4. On the WINDOWS runner grep decided the log was BINARY and printed
#     "Binary file D:\a\_temp/ci-annotated-workspace.log matches" INSTEAD of the
#     matching lines — so the one leg with 17 failures annotated none of their
#     names. `-a` forces text. Observed on run for 7c126db.
#
# Usage:  scripts/ci_run_annotated.sh <label> <command...>
set -uo pipefail

label="${1:?usage: ci_run_annotated.sh <label> <command...>}"
shift
log="${RUNNER_TEMP:-/tmp}/ci-annotated-${label// /-}.log"

set +e
"$@" 2>&1 | tee "$log"
status=${PIPESTATUS[0]}
set -e

if [ "$status" -eq 0 ]; then
  exit 0
fi

n_failed=$(grep -acE '^test .* \.\.\. FAILED$' "$log" || true)

# ── Annotation budget: ~10 per step. Spend it most-informative-first. ─────────
# 1. The summary, ALWAYS, so a truncated list never reads as the whole story.
echo "::error title=${label}: summary::${n_failed} failing test(s); exit status ${status}. Most informative lines first; see the step log for the rest."

emitted=1
BUDGET=9

# 2. Panics WITH the assertion text that follows them. `grep -A1` pairs
#    "panicked at <file:line>:" with the message on the next line, which is where
#    the actual expected-vs-actual lives.
while IFS= read -r pair; do
  [ "$emitted" -ge "$BUDGET" ] && break
  echo "::error title=${label}: panic::${pair}"
  emitted=$((emitted + 1))
done < <(awk '/panicked at /{ line=$0; if ((getline msg) > 0) { gsub(/^[ \t]+/, "", msg); line = line " " msg } print line }' "$log" | sort -u)

# 3. Build/link errors — a compile failure produces NO `test ... FAILED` line at
#    all, so without this the hardest failures to diagnose remotely annotate
#    nothing.
while IFS= read -r line; do
  [ "$emitted" -ge "$BUDGET" ] && break
  echo "::error title=${label}: build error::${line}"
  emitted=$((emitted + 1))
done < <({ grep -aE '^error(\[|:)' "$log" || true; } | sort -u)

# 4. Bare names, only with budget left over.
while IFS= read -r line; do
  [ "$emitted" -ge "$BUDGET" ] && break
  echo "::error title=${label}: test failed::${line}"
  emitted=$((emitted + 1))
done < <({ grep -aE '^test .* \.\.\. FAILED$' "$log" || true; } | sort -u)

# 5. Say so when the cap bit, rather than letting the list look complete.
if [ "$n_failed" -gt 0 ] && [ "$emitted" -ge "$BUDGET" ]; then
  echo "::error title=${label}: elided::annotation budget reached — ${n_failed} test(s) failed in total; this list is truncated."
fi

exit "$status"
