# Kill-Switch Runbook

**Version:** v0  
**Owner:** operator / on-call  
**Related code:** `crates/agent/src/kill_switch.rs`, `crates/audit/src/journal.rs`

---

## Overview

The kill switch is a hard-stop mechanism that immediately halts all order
submission and flags the agent as `Halted`.  It is trip-once (sticky): once
tripped it cannot be reset by the agent process itself — an operator must
intervene.

Trip sources (in priority order):

| Source | Code path | HaltReason |
|--------|-----------|------------|
| Halt file appears on disk | `spawn_halt_file_watcher` polls every 500 ms | `HaltFile` |
| Ledger imbalance detected | `reconciler::check_balance` → `ks.trip()` | `LedgerImbalance` |
| Heartbeat timeout | `spawn_heartbeat_monitor` (future) | `HeartbeatTimeout` |
| Manual operator trip | `write_halt_file` CLI helper | `HaltFile` |

---

## Trigger conditions

### 1. Halt file present at startup or created at runtime

The agent polls `<halt_file>` (default `halt.lock`, configurable in
`config/agent.toml`) every 500 ms.  If the file is detected:

- `KillSwitch::trip(HaltReason::HaltFile)` is called.
- A `AgentMode::Halted { reason: "halt file detected" }` event is broadcast on
  the mode channel.
- All new orders are suppressed; the existing fill pipeline drains.

The halt file check also runs **synchronously at startup** (`check_halt_file()`
in `main()`), so if the file is present when the agent starts it enters `Halted`
immediately without processing any bars.

### 2. Ledger imbalance

At every 1440-bar boundary (≈ once per trading day in 1-minute data) the
backtest reconciler verifies:

```
cash + Σ(position_qty × last_mark) == equity_curve.last()
```

If the absolute difference exceeds `tolerance` (0.01 USDT), the reconciler
increments `ledger_imbalance_events` and emits a `WARN` tracing event.  In the
live/paper agent, the reconciler calls `ks.trip(HaltReason::LedgerImbalance)`.

---

## Expected behaviour after trip

1. `KillSwitch::is_tripped()` returns `true` immediately (atomic store).
2. The mode broadcast channel delivers `AgentMode::Halted { reason }` to all
   subscribers (UI, prometheus, reconciler task).
3. The agent's `tokio::select!` wakes on the mode channel and logs
   `"agent halted"` then falls through to graceful shutdown.
4. No further orders are submitted to the execution router.
5. The audit journal records a `KillSwitchTripped` memo entry via
   `audit::journal::kill_switch_tripped(ledger, reason, operator)`.

---

## Recovery steps

### Step 1 — Diagnose

Check the most recent tracing output (JSON lines to stdout):

```bash
# Last 100 lines of the agent log
journalctl -u trading-agent -n 100 --output json-pretty | grep -i halt
```

Or if running directly:

```bash
RUST_LOG=warn cargo run --bin trading -- --config config/agent.toml 2>&1 | tail -50
```

### Step 2 — Inspect the audit ledger

```sql
-- Find the KillSwitchTripped transaction
SELECT id, ts, description, metadata
FROM journal_transactions
WHERE description LIKE 'registry:KillSwitchTripped:%'
ORDER BY ts DESC
LIMIT 5;

-- Verify global balance is still intact
SELECT
    SUM(CAST(debit_amount  AS REAL)) AS total_dr,
    SUM(CAST(credit_amount AS REAL)) AS total_cr,
    SUM(CAST(debit_amount  AS REAL)) - SUM(CAST(credit_amount AS REAL)) AS imbalance
FROM journal_entries;
-- Expected: |imbalance| < 0.00000001

-- Check last few fills to see if a bad trade triggered the imbalance
SELECT jt.ts, jt.description, je.account_id, je.debit_amount, je.credit_amount
FROM journal_entries je
JOIN journal_transactions jt ON je.transaction_id = jt.id
WHERE jt.description LIKE 'Buy%' OR jt.description LIKE 'Sell%'
ORDER BY jt.ts DESC
LIMIT 20;
```

If `ledger_imbalance_total` in Prometheus is non-zero, identify the bar index
from the tracing `WARN` log (`bar=<idx>`).

### Step 3 — Assess position risk

```sql
-- Current position snapshot (last fill per symbol)
SELECT account_id,
       SUM(CAST(debit_amount  AS REAL)) - SUM(CAST(credit_amount AS REAL)) AS net
FROM journal_entries
GROUP BY account_id
ORDER BY account_id;
```

If `assets:position:BTC` net > 0, there is an open position.  Assess market
risk before resuming.

### Step 4 — Remediate

**If halt was operator-initiated (halt file):**

```bash
# Remove the halt file — this does NOT automatically restart the agent
rm halt.lock
# Then restart manually:
cargo run --bin trading -- --config config/agent.toml
```

**If halt was due to ledger imbalance:**

1. Run the balance query above and confirm the root cause (bug vs market data
   spike vs fee calculation error).
2. Fix the underlying issue in code (open a bug ticket, patch, redeploy).
3. Consider manually flattening any open position before restarting.
4. Only after root cause is understood, remove the halt file and restart.

**Never restart the agent without understanding why it halted.**

### Step 5 — Verify clean restart

After restart, confirm in Prometheus / tracing:

```
ledger_imbalance_total{} == 0
kill_switch_trips_total{} == 0
```

And confirm the ledger balance query above shows `|imbalance| < 1e-8`.

---

## Prometheus alert rules (reference)

```yaml
# Alert if kill switch trips
- alert: AgentKillSwitchTripped
  expr: kill_switch_trips_total > 0
  for: 0m
  labels:
    severity: critical
  annotations:
    summary: "Trading agent kill switch tripped"

# Alert if ledger imbalance detected
- alert: LedgerImbalance
  expr: ledger_imbalance_total > 0
  for: 0m
  labels:
    severity: critical
  annotations:
    summary: "Audit ledger imbalance detected"
```

---

## Clean-flatten procedure (v0.5 placeholder)

In v0 the agent does not submit live orders, so no exchange-side flatten is
required.  In v0.5 (live paper trading), add:

1. Cancel all open orders via `ExecRouter::cancel_all()`.
2. Submit a market sell for `position_qty` via the paper engine.
3. Verify `assets:position:BTC` in the ledger drops to zero.
4. Log the flatten fill to the audit journal as usual.

---

*Last updated: 2026-04-18*
