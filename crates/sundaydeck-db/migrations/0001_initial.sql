-- Migration 0001: Core Presentation, Songs (with FTS5), Themes, Media, Services, Settings

CREATE TABLE media_assets (
    id            TEXT PRIMARY KEY NOT NULL,
    kind          TEXT NOT NULL CHECK (kind IN ('image', 'svg', 'pdf_page', 'logo')),
    name          TEXT NOT NULL,
    sha256        TEXT NOT NULL UNIQUE,
    relative_path TEXT NOT NULL,
    mime_type     TEXT NOT NULL,
    width         INTEGER CHECK (width IS NULL OR width > 0),
    height        INTEGER CHECK (height IS NULL OR height > 0),
    byte_size     INTEGER NOT NULL CHECK (byte_size >= 0),
    created_at    INTEGER NOT NULL
);

CREATE TABLE themes (
    id                  TEXT PRIMARY KEY NOT NULL,
    name                TEXT NOT NULL UNIQUE,
    target              TEXT NOT NULL CHECK (target IN ('song', 'scripture', 'announcement', 'general')),
    is_default          INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
    background_kind     TEXT NOT NULL CHECK (background_kind IN ('solid', 'linear_gradient', 'image')),
    background_json     TEXT NOT NULL,
    font_family         TEXT NOT NULL,
    font_size_pt        REAL NOT NULL CHECK (font_size_pt > 0.0),
    font_weight         INTEGER NOT NULL CHECK (font_weight BETWEEN 100 AND 900),
    line_height         REAL NOT NULL CHECK (line_height > 0.0),
    text_color_rgba     INTEGER NOT NULL,
    h_align             TEXT NOT NULL CHECK (h_align IN ('left', 'center', 'right')),
    v_align             TEXT NOT NULL CHECK (v_align IN ('top', 'middle', 'bottom')),
    stroke_width_px     REAL NOT NULL DEFAULT 0.0 CHECK (stroke_width_px >= 0.0),
    stroke_color_rgba   INTEGER NOT NULL DEFAULT 0,
    shadow_json         TEXT,
    margin_top_pct      REAL NOT NULL DEFAULT 0.06 CHECK (margin_top_pct >= 0.0 AND margin_top_pct < 0.5),
    margin_right_pct    REAL NOT NULL DEFAULT 0.06 CHECK (margin_right_pct >= 0.0 AND margin_right_pct < 0.5),
    margin_bottom_pct   REAL NOT NULL DEFAULT 0.08 CHECK (margin_bottom_pct >= 0.0 AND margin_bottom_pct < 0.5),
    margin_left_pct     REAL NOT NULL DEFAULT 0.06 CHECK (margin_left_pct >= 0.0 AND margin_left_pct < 0.5),
    footer_font_size_pt REAL NOT NULL DEFAULT 24.0 CHECK (footer_font_size_pt > 0.0),
    transition_ms       INTEGER NOT NULL DEFAULT 250 CHECK (transition_ms BETWEEN 0 AND 3000),
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL
);

CREATE UNIQUE INDEX idx_themes_single_default_per_target
    ON themes(target)
    WHERE is_default = 1;

CREATE TABLE songs (
    id                     TEXT PRIMARY KEY NOT NULL,
    title                  TEXT NOT NULL,
    alternate_title        TEXT NOT NULL DEFAULT '',
    author                 TEXT NOT NULL DEFAULT '',
    copyright              TEXT NOT NULL DEFAULT '',
    ccli_number            TEXT NOT NULL DEFAULT '',
    default_theme_id       TEXT REFERENCES themes(id) ON DELETE SET NULL,
    default_arrangement_id TEXT REFERENCES song_arrangements(id) ON DELETE SET NULL DEFERRABLE INITIALLY DEFERRED,
    default_reflow_mode    TEXT NOT NULL DEFAULT 'by_stanza'
                           CHECK (default_reflow_mode IN ('by_stanza', 'lines_1', 'lines_2', 'lines_4')),
    lyrics_search_text     TEXT NOT NULL DEFAULT '',
    created_at             INTEGER NOT NULL,
    updated_at             INTEGER NOT NULL
);

CREATE INDEX idx_songs_title ON songs(title COLLATE NOCASE);

CREATE TABLE song_sections (
    id      TEXT PRIMARY KEY NOT NULL,
    song_id TEXT NOT NULL REFERENCES songs(id) ON DELETE CASCADE,
    kind    TEXT NOT NULL CHECK (
                kind IN ('intro', 'verse', 'pre_chorus', 'chorus', 'bridge', 'tag', 'interlude', 'ending', 'other')
            ),
    label   TEXT NOT NULL,
    ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
    body    TEXT NOT NULL,
    UNIQUE (song_id, ordinal)
);

CREATE INDEX idx_song_sections_song ON song_sections(song_id, ordinal);

CREATE TABLE song_arrangements (
    id         TEXT PRIMARY KEY NOT NULL,
    song_id    TEXT NOT NULL REFERENCES songs(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE (song_id, name)
);

CREATE TABLE song_arrangement_items (
    arrangement_id TEXT NOT NULL REFERENCES song_arrangements(id) ON DELETE CASCADE,
    position       INTEGER NOT NULL CHECK (position >= 0),
    section_id     TEXT NOT NULL REFERENCES song_sections(id) ON DELETE CASCADE,
    PRIMARY KEY (arrangement_id, position)
);

CREATE INDEX idx_arrangement_items_section ON song_arrangement_items(section_id);

CREATE VIRTUAL TABLE songs_fts USING fts5(
    title,
    alternate_title,
    author,
    ccli_number,
    lyrics_search_text,
    content='songs',
    content_rowid='rowid',
    tokenize='unicode61 remove_diacritics 2'
);

CREATE TRIGGER songs_fts_ai AFTER INSERT ON songs BEGIN
    INSERT INTO songs_fts(rowid, title, alternate_title, author, ccli_number, lyrics_search_text)
    VALUES (new.rowid, new.title, new.alternate_title, new.author, new.ccli_number, new.lyrics_search_text);
END;

CREATE TRIGGER songs_fts_ad AFTER DELETE ON songs BEGIN
    INSERT INTO songs_fts(songs_fts, rowid, title, alternate_title, author, ccli_number, lyrics_search_text)
    VALUES ('delete', old.rowid, old.title, old.alternate_title, old.author, old.ccli_number, old.lyrics_search_text);
END;

CREATE TRIGGER songs_fts_au AFTER UPDATE ON songs BEGIN
    INSERT INTO songs_fts(songs_fts, rowid, title, alternate_title, author, ccli_number, lyrics_search_text)
    VALUES ('delete', old.rowid, old.title, old.alternate_title, old.author, old.ccli_number, old.lyrics_search_text);
    INSERT INTO songs_fts(rowid, title, alternate_title, author, ccli_number, lyrics_search_text)
    VALUES (new.rowid, new.title, new.alternate_title, new.author, new.ccli_number, new.lyrics_search_text);
END;

CREATE TABLE services (
    id           TEXT PRIMARY KEY NOT NULL,
    title        TEXT NOT NULL,
    service_date TEXT,
    notes        TEXT NOT NULL DEFAULT '',
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL
);

CREATE INDEX idx_services_updated ON services(updated_at DESC);

CREATE TABLE service_items (
    id                TEXT PRIMARY KEY NOT NULL,
    service_id        TEXT NOT NULL REFERENCES services(id) ON DELETE CASCADE,
    position          INTEGER NOT NULL CHECK (position >= 0),
    kind              TEXT NOT NULL CHECK (
                          kind IN ('song', 'scripture', 'presentation', 'announcement_loop', 'custom_slide')
                      ),
    title             TEXT NOT NULL,
    song_id           TEXT REFERENCES songs(id) ON DELETE SET NULL,
    arrangement_id    TEXT REFERENCES song_arrangements(id) ON DELETE SET NULL,
    theme_override_id TEXT REFERENCES themes(id) ON DELETE SET NULL,
    reflow_override   TEXT CHECK (
                          reflow_override IS NULL
                          OR reflow_override IN ('by_stanza', 'lines_1', 'lines_2', 'lines_4')
                      ),
    payload_json      TEXT NOT NULL DEFAULT '{}',
    UNIQUE (service_id, position)
);

CREATE INDEX idx_service_items_service ON service_items(service_id, position);

CREATE TABLE app_settings (
    key        TEXT PRIMARY KEY NOT NULL,
    value_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
