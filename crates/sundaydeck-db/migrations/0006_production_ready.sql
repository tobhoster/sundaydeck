-- Migration 0006: Production persistence for Custom Slides & CCLI Song Usage Compliance Log.

CREATE TABLE IF NOT EXISTS custom_slides (
    id         TEXT PRIMARY KEY NOT NULL,
    label      TEXT NOT NULL,
    spec_json  TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_custom_slides_updated
    ON custom_slides (updated_at DESC);

CREATE TABLE IF NOT EXISTS ccli_usage_log (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    projected_at INTEGER NOT NULL,
    song_id      TEXT,
    title        TEXT NOT NULL,
    author       TEXT NOT NULL DEFAULT '',
    ccli_number  TEXT NOT NULL DEFAULT '',
    copyright    TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_ccli_usage_projected_at
    ON ccli_usage_log (projected_at DESC);
