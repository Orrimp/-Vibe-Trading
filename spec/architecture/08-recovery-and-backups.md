---
slug: architecture-08-recovery-and-backups
status: shipped
owner: architect
updated: 2026-05-13
---

# Disaster recovery and backups

v0 → v3 policy: **local snapshots only.** No cloud spend until the
project reaches terminal state. Confirmed 2026-04-19
([../product.md → Open decisions](../product.md#open-decisions)).

## Backup surface

- **SQLite ledger:** `sqlite3 <db> ".backup 'snapshot/<YYYY-MM-DD>-ledger.db'"`
  nightly via a tokio task in the `agent` binary. Retain 30 days; purge
  older.
- **Parquet archive:** historical market data lives in `data/binance/...`;
  treated as append-only. Weekly `rsync -a data/ data-snapshot/` rotation
  gives a 4-week rolling local backup.
- **Config + strategy TOML:** versioned in place under `config/`;
  backed up alongside the ledger snapshot.

## RPO / RTO

- **RPO:** 24h (ledger + config).
- **RTO:** ~1h manual (copy snapshot, restart agent).

## Explicitly out of scope

Off-site cloud sync (B2 / S3), continuous WAL streaming (`litestream`),
multi-region replication. Deferred to a follow-up project triggered when
real-money execution lands
([../product.md → Project scope boundary](../product.md#project-scope-boundary)).

## Runbook

Restore runbook lives at `../runbooks/disaster-recovery.md` (v0.5
deliverable — file not yet created; for v0 a section in
[`../../docs/runbooks/kill-switch.md`](../../docs/runbooks/kill-switch.md) suffices).

## Changelog
- 2026-05-13 (architect): content migrated from `spec/architecture.md` §
  Disaster recovery & backups during Phase 1A Session 3. Two link
  rewrites applied: `product.md` → `../product.md` and
  `docs/runbooks/...` → `../runbooks/...` (relative paths from the new
  location).
