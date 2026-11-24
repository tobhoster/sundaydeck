-- Migration 0004: Allow `live_camera` BackgroundSource and persist structured `SlideTransition`.

CREATE TABLE themes_v4 (
    id                  TEXT PRIMARY KEY NOT NULL,
    name                TEXT NOT NULL UNIQUE,
    target              TEXT NOT NULL CHECK (target IN ('song', 'scripture', 'announcement', 'general')),
    is_default          INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
    background_kind     TEXT NOT NULL CHECK (background_kind IN ('solid', 'linear_gradient', 'image', 'video_loop', 'live_camera')),
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
    transition_json     TEXT,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL
);

INSERT INTO themes_v4 (
    id, name, target, is_default, background_kind, background_json,
    font_family, font_size_pt, font_weight, line_height, text_color_rgba,
    h_align, v_align, stroke_width_px, stroke_color_rgba, shadow_json,
    margin_top_pct, margin_right_pct, margin_bottom_pct, margin_left_pct,
    footer_font_size_pt, transition_ms, transition_json, created_at, updated_at
)
SELECT
    id, name, target, is_default, background_kind, background_json,
    font_family, font_size_pt, font_weight, line_height, text_color_rgba,
    h_align, v_align, stroke_width_px, stroke_color_rgba, shadow_json,
    margin_top_pct, margin_right_pct, margin_bottom_pct, margin_left_pct,
    footer_font_size_pt, transition_ms, NULL, created_at, updated_at
FROM themes;

DROP TABLE themes;
ALTER TABLE themes_v4 RENAME TO themes;

CREATE UNIQUE INDEX idx_themes_single_default_per_target
    ON themes(target)
    WHERE is_default = 1;
