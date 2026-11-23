-- Migration 0002: Multi-Translation Offline Bible Store + FTS5 Search

CREATE TABLE bible_translations (
    id         TEXT PRIMARY KEY NOT NULL,
    code       TEXT NOT NULL UNIQUE,
    name       TEXT NOT NULL,
    language   TEXT NOT NULL DEFAULT 'en',
    copyright  TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL
);

CREATE TABLE bible_books (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    translation_id TEXT NOT NULL REFERENCES bible_translations(id) ON DELETE CASCADE,
    book_number    INTEGER NOT NULL CHECK (book_number BETWEEN 1 AND 66),
    name           TEXT NOT NULL,
    chapter_count  INTEGER NOT NULL CHECK (chapter_count >= 1),
    UNIQUE (translation_id, book_number)
);

CREATE TABLE bible_verses (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    translation_id TEXT NOT NULL REFERENCES bible_translations(id) ON DELETE CASCADE,
    book_number    INTEGER NOT NULL CHECK (book_number BETWEEN 1 AND 66),
    chapter        INTEGER NOT NULL CHECK (chapter >= 1),
    verse          INTEGER NOT NULL CHECK (verse >= 1),
    text           TEXT NOT NULL,
    UNIQUE (translation_id, book_number, chapter, verse)
);

CREATE INDEX idx_bible_verses_lookup
    ON bible_verses(translation_id, book_number, chapter, verse);

CREATE VIRTUAL TABLE bible_verses_fts USING fts5(
    text,
    content='bible_verses',
    content_rowid='id',
    tokenize='unicode61 remove_diacritics 2'
);

CREATE TRIGGER bible_verses_fts_ai AFTER INSERT ON bible_verses BEGIN
    INSERT INTO bible_verses_fts(rowid, text) VALUES (new.id, new.text);
END;

CREATE TRIGGER bible_verses_fts_ad AFTER DELETE ON bible_verses BEGIN
    INSERT INTO bible_verses_fts(bible_verses_fts, rowid, text) VALUES ('delete', old.id, old.text);
END;

CREATE TRIGGER bible_verses_fts_au AFTER UPDATE ON bible_verses BEGIN
    INSERT INTO bible_verses_fts(bible_verses_fts, rowid, text) VALUES ('delete', old.id, old.text);
    INSERT INTO bible_verses_fts(rowid, text) VALUES (new.id, new.text);
END;
