---
title: Operator Runbook — Real-Time Paper Soak (90-day longevity)
version: 1.0.0
author: developer-agent
date: 2026-06-19
applies_to: trading v0.1.0+
---

# Operator Runbook: Real-Time Multi-Day Paper Soak

This is a self-contained recipe for running a genuine multi-day (target: 90-day)
real-time paper soak of the trading agent. It covers setup, monitoring, restart
procedure, and evidence capture.

---

## Prerequisites

- macOS or Linux host with stable internet connection (for live Binance WS feed)
- Rust toolchain (stable, edition 2024) — `rustup show` to confirm
- SQLite CLI available: `sqlite3 --version`
- At minimum 200 MB free disk space for audit DB growth over 90 days
- A dedicated scratch directory (suggested: `/var/data/soak/` or `/tmp/soak/`)

No API keys required — LLM is disabled, Binance feed is WS read-only (no auth
needed for public market data streams).

---

## Command

Primary command to start or restart the soak:

```bash
cargo run --release --bin trading -- \
  --config config/agent.toml.soak \
  --mode paper \
  2>&1 | tee -a /tmp/soak.log
```

**Do NOT use `--fast-replay` in real-time mode.** The `--fast-replay` flag only
affects `replay_pace_ms` in research mode; in paper mode it is a no-op. For the
real-time soak, omit it entirely so bars arrive at the natural 1-minute cadence from
Binance WS.

Alternative: run in a `screen` or `tmux` session so the process survives terminal
disconnection:

```bash
screen -S paper-soak
cargo run --release --bin trading -- \
  --config config/agent.toml.soak \
  --mode paper \
  2>&1 | tee -a /tmp/soak.log
# Detach: Ctrl-A D
# Reattach: screen -r paper-soak
```

---

## Steps

### Step 1: Prepare the soak config

The soak config at `config/agent.toml.soak` is committed to the repository.
Key parameters are:

- `mode = "paper"` (live Binance WS, paper fills — no real orders)
- `ledger_db_path = "/tmp/soak-audit.db"` (scratch DB — does NOT write to data/audit/ledger.db)
- `reflection.path = "/tmp/soak-reflection.db"` (scratch)
- `llm.enabled = false` (no API keys needed)
- `strategies.sma_crossover.enabled = true` (deterministic, no external deps)
- `kill_switch.halt_file = "/tmp/.soak-halt"`
- `prometheus_enabled = false` (headless)

If you want the DB in a durable location (recommended for 90-day soak):

```bash
# Edit only the ledger_db_path and reflection.path in the soak config:
sed -i 's|/tmp/soak-audit.db|/var/data/soak/audit.db|g' config/agent.toml.soak
sed -i 's|/tmp/soak-reflection.db|/var/data/soak/reflection.db|g' config/agent.toml.soak
mkdir -p /var/data/soak
```

Or create a local override at `config/agent.toml.soak.local` (git-ignored pattern).

### Step 2: First boot

```bash
# Ensure no stale halt file from a previous run
rm -f /tmp/.soak-halt

# Start the soak (foreground, tee to log)
cargo run --release --bin trading -- \
  --config config/agent.toml.soak \
  --mode paper \
  2>&1 | tee /tmp/soak.log
```

Confirm the startup sequence in the log:

- `"trading agent starting"` — binary launched
- `"audit ledger initialized","db":"/tmp/soak-audit.db"` — scratch DB opened
- `"reflection writer task spawned"` — lesson card writer active
- `"agent uptime interval opened","boot_id":"<uuid>"` — uptime tracking started
- `"llm subsystem disabled"` — confirmed no LLM keys needed
- `"paper trading loop spawned"` — paper engine running
- `"Binance feed initialized"` — WS URL logged
- `"bars_tap started","symbol":"BTCUSDT","tf":"1m"` — receiving market data
- `"ticks_tap started","symbol":"BTCUSDT"` — tick feed active

If any of these lines are absent, check the log for ERROR lines before proceeding.

### Step 3: Verify data is flowing (within 2 minutes of start)

```bash
# Equity snapshots should appear at 1-minute intervals
sqlite3 /tmp/soak-audit.db "SELECT count(*) FROM equity_snapshots;"
# Expected: 1+ rows (grows by 1 per minute)

# Uptime heartbeat should be updating
sqlite3 /tmp/soak-audit.db "SELECT boot_id, started_at, last_heartbeat_at FROM agent_uptime ORDER BY started_at DESC LIMIT 1;"
# Expected: last_heartbeat_at advances each time you query

# Kill switch should NOT have tripped
sqlite3 /tmp/soak-audit.db "SELECT count(*) FROM strategy_events WHERE event_type = 'kill_switch_trip';" 2>/dev/null
# Expected: 0
```

### Step 4: Monitor during the soak

Run this in a separate terminal (copy-paste ready):

```bash
watch -n 60 '
PID=$(pgrep -f "trading --config config/agent.toml.soak" | head -1)
[ -z "$PID" ] && echo "AGENT NOT RUNNING — check /tmp/soak.log" && tail -5 /tmp/soak.log && exit
EQUITY=$(sqlite3 /tmp/soak-audit.db "SELECT count(*) FROM equity_snapshots;" 2>/dev/null)
UPTIME_ROW=$(sqlite3 /tmp/soak-audit.db "SELECT started_at, last_heartbeat_at FROM agent_uptime ORDER BY started_at DESC LIMIT 1;" 2>/dev/null)
ELAPSED=$(ps -o etime= -p $PID | awk "{gsub(/^ +/,\"\"); n=split(\$0,a,/[-:]/); if(n==2)print a[1]*60+a[2]; else if(n==3)print a[1]*3600+a[2]*60+a[3]; else if(n==4)print a[1]*86400+a[2]*3600+a[3]*60+a[4]}")
echo "PID=$PID elapsed=${ELAPSED}s equity_snapshots=$EQUITY"
echo "uptime_row: $UPTIME_ROW"
tail -n 3 /tmp/soak.log 2>/dev/null
'
```

### Step 5: Graceful restart procedure

If the agent needs to restart (host reboot, maintenance, etc.):

```bash
# 1. Send SIGINT (graceful shutdown — closes uptime interval)
PID=$(pgrep -f "trading --config config/agent.toml.soak" | head -1)
kill -INT $PID

# 2. Wait for clean exit (typically < 5 seconds)
until ! pgrep -f "trading --config config/agent.toml.soak" > /dev/null 2>&1; do sleep 1; done
echo "Agent stopped cleanly"

# 3. Verify stopped_at was written
sqlite3 /tmp/soak-audit.db "SELECT boot_id, started_at, stopped_at FROM agent_uptime ORDER BY started_at DESC LIMIT 1;"
# Expected: stopped_at IS NOT NULL

# 4. Remove any stale halt file
rm -f /tmp/.soak-halt

# 5. Restart against the SAME DB (state resumes)
cargo run --release --bin trading -- \
  --config config/agent.toml.soak \
  --mode paper \
  2>&1 | tee -a /tmp/soak.log
```

The second boot opens the same DB, reads existing equity snapshots, and continues
from the durable state. The `agent_uptime` table will have a new row for the new
boot_id; the old rows remain for uptime calculation.

**Important:** use SIGINT (kill -INT), not SIGTERM (kill -TERM) or SIGKILL (kill -9).
SIGTERM exits the process before `shutdown_writer` runs, leaving `stopped_at = NULL`
on the uptime row. SIGKILL is always unclean. Only SIGINT (Ctrl-C / kill -INT)
triggers the graceful tokio Ctrl-C handler.

### Step 6: Generate the operator-success report

At any point during or after the soak, generate the report:

```bash
# 7-day window
cargo run --release --bin report -- \
  --period 7d \
  --ledger /tmp/soak-audit.db \
  --output spec/paper-soak-longevity/reports/operator-success-soak-7d-$(date +%Y-%m-%d).md \
  --seed 0xC0FFEE

# 30-day window
cargo run --release --bin report -- \
  --period 30d \
  --ledger /tmp/soak-audit.db \
  --output spec/paper-soak-longevity/reports/operator-success-soak-30d-$(date +%Y-%m-%d).md \
  --seed 0xC0FFEE

# Full inception-to-date
cargo run --release --bin report -- \
  --period inception \
  --ledger /tmp/soak-audit.db \
  --output spec/paper-soak-longevity/reports/operator-success-soak-inception-$(date +%Y-%m-%d).md \
  --seed 0xC0FFEE
```

The report includes:
- `reconciliation: PASS/FAIL` — double-entry balance integrity
- `Kill-switch trips: N` — expected 0 during normal operation
- `Uptime %` — fraction of elapsed window with at least one running boot
- Equity curve sparkline
- Risk metrics (Sharpe, Sortino, Calmar, max drawdown) — meaningful after 50+ bars

### Step 7: Capture the 90-day evidence

At the end of the soak period:

```bash
# Query uptime statistics
sqlite3 /tmp/soak-audit.db "
SELECT 
  count(*) as total_boots,
  sum(CASE WHEN stopped_at IS NOT NULL THEN 1 ELSE 0 END) as clean_shutdowns,
  min(started_at) as first_boot,
  max(COALESCE(stopped_at, last_heartbeat_at)) as last_activity
FROM agent_uptime;
"

# Query equity snapshot count (should be ~1 per minute = ~129,600 for 90 days)
sqlite3 /tmp/soak-audit.db "SELECT count(*), min(ts), max(ts) FROM equity_snapshots;"

# Generate final report
cargo run --release --bin report -- \
  --period 90d \
  --ledger /tmp/soak-audit.db \
  --output spec/paper-soak-longevity/reports/operator-success-soak-90d-final.md \
  --seed 0xC0FFEE
```

---

## Timing

| Phase | Duration |
|-------|----------|
| Build (first time) | ~40 seconds |
| Boot to first market data | ~2 seconds |
| SMA warmup (50 bars) | ~50 minutes |
| First fill generated | ~50-60 minutes after boot |
| Evidence accumulation window | 90 days (target) |
| Report generation | < 1 second |

---

## Expected Results

After 90 days of continuous operation:

| Metric | Expected |
|--------|----------|
| Uptime % | >99% (at most 2 hours downtime over 90 days) |
| Kill-switch trips | 0 (no risk breaches in normal market conditions) |
| Equity snapshots | ~129,600 rows (1 per minute) |
| Agent uptime rows | 1-3 (clean boot, maybe 1-2 planned restarts) |
| Reconciliation | PASS every report run |
| Feed reconnects | <10 (transient Binance WS drops recovered) |
| LLM spend | $0.00 (disabled) |

---

## Failure Diagnosis

### Agent is not running (process not found)

```bash
tail -50 /tmp/soak.log | grep -E "ERROR|WARN|panic"
```

Common causes:
- Kill switch tripped: `ls /tmp/.soak-halt` — if file exists, remove it and restart
- Port conflict (prometheus): already disabled in soak config, should not happen
- DB file permissions: `ls -la /tmp/soak-audit.db`

### Kill switch tripped

```bash
sqlite3 /tmp/soak-audit.db "SELECT * FROM strategy_events WHERE event_type = 'kill_switch_trip';"
```

If a trip occurred, review the reason. For paper mode, the kill switch trips on:
- `max_drawdown_stop_pct` exceeded (-15% by default) — equity fell > 15% below peak
- `daily_loss_stop_pct` exceeded (-5% by default) — single-day loss > 5%

To reset: remove `/tmp/.soak-halt` and restart. Adjust risk parameters in
`config/agent.toml.soak` if needed.

### No equity snapshots after 2 minutes

The equity snapshot task runs at 1-minute intervals. If after 2 minutes there are
still 0 rows:

```bash
# Check if the equity purge task is running (look for the log line)
grep "equity_purge_task started" /tmp/soak.log
# Check for any ERROR lines in the log
grep '"level":"ERROR"' /tmp/soak.log
```

### Reconciliation FAIL in the report

The report exits with code 1 on reconciliation failure. The sibling failure report
is written to `<output>-reconciliation-fail.md`. Review:

```bash
cat spec/paper-soak-longevity/reports/operator-success-soak-inception-<date>-reconciliation-fail.md
```

Reconciliation failures indicate a double-entry accounting bug — escalate to the
development team.

### Binance WS disconnects

The agent automatically reconnects. Feed reconnects are logged:
```bash
grep "feed_reconnect\|reconnect" /tmp/soak.log | tail -20
```

If reconnect rate is high (>10/day), check the Binance API status page and your
network stability.

---

## Cleanup

After the soak is complete and evidence is captured:

```bash
# Stop the agent
PID=$(pgrep -f "trading --config config/agent.toml.soak" | head -1)
[ -n "$PID" ] && kill -INT $PID

# Archive the evidence (optional — keep for audit trail)
cp /tmp/soak-audit.db spec/paper-soak-longevity/soak-audit-$(date +%Y-%m-%d).db.bak

# Remove scratch files (only after evidence is safely copied)
rm -f /tmp/soak-audit.db /tmp/soak-reflection.db /tmp/.soak-halt /tmp/soak.log
```

Do NOT delete the generated report files under `spec/paper-soak-longevity/reports/`.

---

## Watch Recipe (operator-side terminal monitoring)

```bash
watch -n 30 '
PID=$(pgrep -f "trading --config config/agent.toml.soak" | head -1)
[ -z "$PID" ] && echo "AGENT NOT RUNNING" && exit
EQUITY_COUNT=$(sqlite3 /tmp/soak-audit.db "SELECT count(*) FROM equity_snapshots;" 2>/dev/null || echo "?")
UPTIME_BEAT=$(sqlite3 /tmp/soak-audit.db "SELECT last_heartbeat_at FROM agent_uptime ORDER BY started_at DESC LIMIT 1;" 2>/dev/null || echo "?")
TRIPS=$(sqlite3 /tmp/soak-audit.db "SELECT count(*) FROM strategy_events WHERE event_type = '"'"'kill_switch_trip'"'"';" 2>/dev/null || echo "?")
ELAPSED=$(ps -o etime= -p $PID | awk "{gsub(/^ +/,\"\"); n=split(\$0,a,/[-:]/); if(n==4)print a[1]\"d \"a[2]\"h \"a[3]\"m\"; else if(n==3)print a[1]\"h \"a[2]\"m\"; else if(n==2)print a[1]\"m\"}")
echo "PID=$PID | elapsed=$ELAPSED | equity_rows=$EQUITY_COUNT | kill_trips=$TRIPS"
echo "last_heartbeat=$UPTIME_BEAT"
tail -n 2 /tmp/soak.log 2>/dev/null
'
```
