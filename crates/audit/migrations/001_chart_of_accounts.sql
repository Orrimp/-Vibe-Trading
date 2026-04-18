-- Chart of accounts (R3.2)
-- Every account is identified by its hierarchical path, e.g. "assets:cash:USDT".

CREATE TABLE IF NOT EXISTS accounts (
    id      TEXT PRIMARY KEY NOT NULL,   -- e.g. "assets:cash:USDT"
    kind    TEXT NOT NULL,               -- "asset" | "income" | "expense" | "equity" | "liability"
    currency TEXT NOT NULL DEFAULT 'USDT', -- ISO code or crypto ticker
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- Journal transactions (each transaction has a unique id)
CREATE TABLE IF NOT EXISTS journal_transactions (
    id          TEXT PRIMARY KEY NOT NULL,  -- UUID
    ts          TEXT NOT NULL,              -- ISO 8601 UTC
    description TEXT NOT NULL DEFAULT '',
    metadata    TEXT NOT NULL DEFAULT '{}'  -- JSON blob for audit metadata
);

-- Journal entries (double-entry lines, must balance per transaction)
CREATE TABLE IF NOT EXISTS journal_entries (
    id             TEXT PRIMARY KEY NOT NULL,  -- UUID
    transaction_id TEXT NOT NULL REFERENCES journal_transactions(id),
    account_id     TEXT NOT NULL REFERENCES accounts(id),
    -- Amounts stored as text to preserve Decimal precision
    debit_amount   TEXT NOT NULL DEFAULT '0',
    credit_amount  TEXT NOT NULL DEFAULT '0',
    ts             TEXT NOT NULL,
    memo           TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_entries_account ON journal_entries(account_id);
CREATE INDEX IF NOT EXISTS idx_entries_ts      ON journal_entries(ts);
CREATE INDEX IF NOT EXISTS idx_entries_txn     ON journal_entries(transaction_id);
