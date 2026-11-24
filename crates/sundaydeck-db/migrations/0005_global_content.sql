-- Migration 0005: Global Content Architecture
-- 73-book Catholic + 66-book Protestant USFM canon, diacritic-insensitive FTS5,
-- numbered denominational hymnbooks, bilingual lyrics, call-and-response liturgy,
-- and community .sdeck-pack attribution registry.

CREATE TABLE bible_books_new (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    translation_id TEXT NOT NULL REFERENCES bible_translations(id) ON DELETE CASCADE,
    book_number    INTEGER NOT NULL CHECK (book_number BETWEEN 1 AND 75),
    usfm_code      TEXT NOT NULL DEFAULT '',
    name           TEXT NOT NULL,
    chapter_count  INTEGER NOT NULL CHECK (chapter_count >= 1),
    UNIQUE (translation_id, book_number)
);

INSERT INTO bible_books_new (id, translation_id, book_number, usfm_code, name, chapter_count)
SELECT id, translation_id, book_number, '', name, chapter_count
FROM bible_books;

DROP TABLE bible_books;
ALTER TABLE bible_books_new RENAME TO bible_books;

DROP TRIGGER IF EXISTS bible_verses_fts_ai;
DROP TRIGGER IF EXISTS bible_verses_fts_ad;
DROP TRIGGER IF EXISTS bible_verses_fts_au;
DROP TABLE IF EXISTS bible_verses_fts;

CREATE TABLE bible_verses_new (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    translation_id    TEXT NOT NULL REFERENCES bible_translations(id) ON DELETE CASCADE,
    book_number       INTEGER NOT NULL CHECK (book_number BETWEEN 1 AND 75),
    chapter           INTEGER NOT NULL CHECK (chapter >= 1),
    verse             INTEGER NOT NULL CHECK (verse >= 1),
    text              TEXT NOT NULL,
    search_text_ascii TEXT NOT NULL DEFAULT '',
    UNIQUE (translation_id, book_number, chapter, verse)
);

INSERT INTO bible_verses_new (id, translation_id, book_number, chapter, verse, text, search_text_ascii)
SELECT id, translation_id, book_number, chapter, verse, text, text
FROM bible_verses;

DROP TABLE bible_verses;
ALTER TABLE bible_verses_new RENAME TO bible_verses;

CREATE INDEX idx_bible_verses_lookup
    ON bible_verses(translation_id, book_number, chapter, verse);

CREATE VIRTUAL TABLE bible_verses_fts USING fts5(
    text,
    search_text_ascii,
    content='bible_verses',
    content_rowid='id',
    tokenize='unicode61 remove_diacritics 2'
);

CREATE TRIGGER bible_verses_fts_ai AFTER INSERT ON bible_verses BEGIN
    INSERT INTO bible_verses_fts(rowid, text, search_text_ascii)
    VALUES (new.id, new.text, new.search_text_ascii);
END;

CREATE TRIGGER bible_verses_fts_ad AFTER DELETE ON bible_verses BEGIN
    INSERT INTO bible_verses_fts(bible_verses_fts, rowid, text, search_text_ascii)
    VALUES ('delete', old.id, old.text, old.search_text_ascii);
END;

CREATE TRIGGER bible_verses_fts_au AFTER UPDATE ON bible_verses BEGIN
    INSERT INTO bible_verses_fts(bible_verses_fts, rowid, text, search_text_ascii)
    VALUES ('delete', old.id, old.text, old.search_text_ascii);
    INSERT INTO bible_verses_fts(rowid, text, search_text_ascii)
    VALUES (new.id, new.text, new.search_text_ascii);
END;

INSERT INTO bible_verses_fts(bible_verses_fts) VALUES ('rebuild');

ALTER TABLE songs ADD COLUMN primary_lang TEXT NOT NULL DEFAULT 'en';
ALTER TABLE songs ADD COLUMN secondary_lang TEXT;
ALTER TABLE songs ADD COLUMN traditions_json TEXT NOT NULL DEFAULT '[]';

ALTER TABLE song_sections ADD COLUMN secondary_body TEXT;
ALTER TABLE song_sections ADD COLUMN stage_chords TEXT;

CREATE TABLE hymnals (
    code       TEXT PRIMARY KEY NOT NULL,
    name       TEXT NOT NULL,
    tradition  TEXT NOT NULL DEFAULT '',
    language   TEXT NOT NULL DEFAULT 'en',
    created_at INTEGER NOT NULL
);

CREATE TABLE song_hymnal_entries (
    song_id      TEXT NOT NULL REFERENCES songs(id) ON DELETE CASCADE,
    hymnal_code  TEXT NOT NULL REFERENCES hymnals(code) ON DELETE CASCADE,
    hymn_number  INTEGER NOT NULL CHECK (hymn_number >= 0),
    PRIMARY KEY (song_id, hymnal_code, hymn_number)
);

CREATE INDEX idx_song_hymnal_lookup
    ON song_hymnal_entries(hymnal_code, hymn_number);

CREATE TABLE liturgy_templates (
    id         TEXT PRIMARY KEY NOT NULL,
    title      TEXT NOT NULL,
    tradition  TEXT NOT NULL DEFAULT '',
    language   TEXT NOT NULL DEFAULT 'en',
    created_at INTEGER NOT NULL
);

CREATE TABLE liturgy_lines (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    template_id    TEXT NOT NULL REFERENCES liturgy_templates(id) ON DELETE CASCADE,
    position       INTEGER NOT NULL,
    speaker        TEXT NOT NULL,
    text           TEXT NOT NULL,
    secondary_text TEXT
);

CREATE TABLE content_packs (
    pack_id      TEXT PRIMARY KEY NOT NULL,
    name         TEXT NOT NULL,
    version      TEXT NOT NULL,
    tradition    TEXT NOT NULL DEFAULT '',
    license      TEXT NOT NULL,
    attribution  TEXT NOT NULL DEFAULT '',
    installed_at INTEGER NOT NULL
);
