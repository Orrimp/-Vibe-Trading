#!/usr/bin/env bash
# Story 1-26 — the regeneration driver.
#
# Runs the 34 contaminated θ-surfaces one at a time, resumably, without making the
# operator's machine unusable. Reads WHAT to run from a manifest; this script owns only
# HOW.
#
# ── Why this exists at all ────────────────────────────────────────────────────
#
# The regeneration is 15-20 h of compute on a 14-core box, and every one of its three
# known hazards is the kind that is cheap to prevent here and expensive to discover at
# hour nine:
#
#   1. `param_robustness_sweep`'s DEFAULT --out-dir points INTO the anchored corpus
#      (1-26 AC3). A measurement run was launched on defaults 2026-08-16 and killed
#      mid-compute; `evidence/` stayed clean only because the sweep is slow. This script
#      refuses to run without an explicit out-dir and refuses one that resolves inside
#      `evidence/`.
#   2. A 15-hour job that cannot resume loses everything to one Ctrl-C, one power
#      failure, or one operator who needs their laptop. Each surface is its own
#      invocation, so completed ones are skipped on the next start.
#   3. One surface already saturates ~12.4 cores (measured 2026-08-16: 1087 s wall,
#      13 482 s user). Left alone it takes the whole machine for two days.
#
# ── Usage ─────────────────────────────────────────────────────────────────────
#
#   scripts/relock/run_surfaces.sh --out-dir evidence/v2/harness-relock/reports [--dry-run]
#
# Stop it any time with Ctrl-C; run the same command again to continue.
#
# ── Tuning ────────────────────────────────────────────────────────────────────
#
#   RELOCK_THREADS   rayon threads per surface. Default 8 (operator's choice,
#                    2026-09-25): leaves ~6 of 14 cores free. Wall-clock scales
#                    roughly inversely — 8 threads is about 1.55x the 12.4-thread
#                    floor, so plan ~16 h against a 10.3 h floor, ~23-31 h realistic.
#   RELOCK_NICE      scheduling niceness. Default 19 (lowest priority). This is the
#                    cheap half of the throttle: it costs almost nothing while you are
#                    away and hands the cycles back the moment you start typing.
#
set -euo pipefail

MANIFEST="${MANIFEST:-$(dirname "$0")/surfaces.tsv}"
THREADS="${RELOCK_THREADS:-8}"
NICENESS="${RELOCK_NICE:-19}"
OUT_DIR=""
DRY_RUN=0

while [ $# -gt 0 ]; do
  case "$1" in
    --out-dir) OUT_DIR="${2:-}"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    -h|--help) sed -n '2,40p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

# ── Guard 1: an out-dir is mandatory, and it must not be the anchored corpus ──
if [ -z "$OUT_DIR" ]; then
  cat >&2 <<'MSG'
REFUSED: --out-dir is mandatory.

`param_robustness_sweep`'s own default writes INTO evidence/v1/…/reports/, i.e. into the
anchored corpus. 1-26 AC3 exists because that nearly happened on 2026-08-16. This script
will not run without you saying where the output goes.
MSG
  exit 2
fi
case "$(cd "$(dirname "$OUT_DIR")" 2>/dev/null && pwd -P || echo "$OUT_DIR")/$(basename "$OUT_DIR")" in
  */evidence/v1/*|*/evidence/v2/*|*/evidence/v3/*)
    # A NEW namespace under evidence/ is the intended destination (ADR-0038 D6), but an
    # EXISTING one is not. Refuse only if the leaf already holds anchored bodies.
    if compgen -G "$OUT_DIR/*.md" > /dev/null 2>&1; then
      echo "REFUSED: $OUT_DIR already contains .md reports." >&2
      echo "Regeneration goes to a NEW namespace; old rows stay byte-frozen (ADR-0038 D6)." >&2
      exit 2
    fi
    ;;
esac

if [ ! -f "$MANIFEST" ]; then
  echo "REFUSED: no manifest at $MANIFEST" >&2
  echo "The 34 invocations must be DERIVED and reviewed before any of this runs." >&2
  exit 2
fi

mkdir -p "$OUT_DIR"
LOG_DIR="$OUT_DIR/../logs"; mkdir -p "$LOG_DIR"
PROGRESS="$LOG_DIR/progress.tsv"
[ -f "$PROGRESS" ] || printf 'scenario\tstatus\tseconds\tfinished_utc\n' > "$PROGRESS"

total=$(grep -vcE '^\s*(#|$)' "$MANIFEST")
done_count=0; skipped=0; failed=0; i=0

echo "surfaces: $total · threads: $THREADS · nice: $NICENESS · out: $OUT_DIR"
echo "resumable — Ctrl-C any time, re-run the same command to continue"
echo

while IFS=$'\t' read -r scenario binary args; do
  case "$scenario" in ''|\#*) continue ;; esac
  i=$((i + 1))

  # ── Guard 2: resumability. A surface that already produced a report is done. ──
  if compgen -G "$OUT_DIR/*${scenario}*.md" > /dev/null 2>&1; then
    echo "[$i/$total] SKIP  $scenario (already present)"
    skipped=$((skipped + 1))
    continue
  fi

  cmd=(nice -n "$NICENESS" ./target/release/"$binary")
  # shellcheck disable=SC2206
  cmd+=($args --out-dir "$OUT_DIR")

  if [ "$DRY_RUN" -eq 1 ]; then
    echo "[$i/$total] DRY   RAYON_NUM_THREADS=$THREADS ${cmd[*]}"
    continue
  fi

  echo "[$i/$total] RUN   $scenario"
  start=$(date +%s)
  if RAYON_NUM_THREADS="$THREADS" "${cmd[@]}" > "$LOG_DIR/$scenario.log" 2>&1; then
    secs=$(( $(date +%s) - start ))
    printf '%s\tok\t%s\t%s\n' "$scenario" "$secs" "$(date -u +%FT%TZ)" >> "$PROGRESS"
    echo "      done in $((secs / 60)) min"
    done_count=$((done_count + 1))
  else
    secs=$(( $(date +%s) - start ))
    printf '%s\tFAILED\t%s\t%s\n' "$scenario" "$secs" "$(date -u +%FT%TZ)" >> "$PROGRESS"
    echo "      FAILED after $((secs / 60)) min — see $LOG_DIR/$scenario.log" >&2
    failed=$((failed + 1))
    # Keep going: one bad surface should not cost the other 33. The summary is loud.
  fi
done < "$MANIFEST"

echo
echo "ran $done_count · skipped $skipped · failed $failed · of $total"
[ "$failed" -eq 0 ] || { echo "SOME SURFACES FAILED — see $PROGRESS" >&2; exit 1; }
