use crate::ids::{
    ArrangementId, SUNDAYDECK_NAMESPACE, SectionId, ServiceId, ServiceItemId, SongId, ThemeId,
};
use crate::service::{ServiceItem, ServiceItemPayload, ServiceSchedule};
use crate::song::{Arrangement, ReflowMode, SectionKind, Song, SongSection};
use crate::theme::{
    BackgroundSpec, HorizontalAlign, Rgba8, SafeMargins, TextShadow, Theme, ThemeTarget,
    VerticalAlign,
};

fn v5_theme(tag: &str) -> ThemeId {
    ThemeId::from_v5(
        &SUNDAYDECK_NAMESPACE,
        format!("seed:theme:{tag}").as_bytes(),
    )
}

fn v5_song(tag: &str) -> SongId {
    SongId::from_v5(&SUNDAYDECK_NAMESPACE, format!("seed:song:{tag}").as_bytes())
}

fn v5_section(tag: &str) -> SectionId {
    SectionId::from_v5(
        &SUNDAYDECK_NAMESPACE,
        format!("seed:section:{tag}").as_bytes(),
    )
}

fn v5_arrangement(tag: &str) -> ArrangementId {
    ArrangementId::from_v5(&SUNDAYDECK_NAMESPACE, format!("seed:arr:{tag}").as_bytes())
}

/// Built-in Phase 1 themes seeded on fresh startup.
pub fn default_themes() -> Vec<Theme> {
    vec![
        Theme {
            id: v5_theme("sanctuary-dark"),
            name: "Classic Sanctuary Dark".to_owned(),
            target: ThemeTarget::Song,
            is_default: true,
            background: BackgroundSpec::LinearGradient {
                start: Rgba8::from_rgb(13, 21, 38),
                end: Rgba8::from_rgb(6, 10, 19),
                angle_deg: 160.0,
            },
            font_family: ".SystemUIFont".to_owned(),
            font_size_pt: 76.0,
            font_weight: 700,
            line_height: 1.32,
            text_color: Rgba8::from_rgb(248, 250, 252),
            h_align: HorizontalAlign::Center,
            v_align: VerticalAlign::Middle,
            stroke_width_px: 2.0,
            stroke_color: Rgba8::from_rgba(0, 0, 0, 160),
            shadow: Some(TextShadow {
                offset_x_px: 0.0,
                offset_y_px: 6.0,
                blur_radius_px: 14.0,
                color: Rgba8::from_rgba(0, 0, 0, 190),
            }),
            margins: SafeMargins::DEFAULT,
            footer_font_size_pt: 24.0,
            transition_ms: 280,
        },
        Theme {
            id: v5_theme("warm-amber-glow"),
            name: "Warm Sanctuary Amber".to_owned(),
            target: ThemeTarget::General,
            is_default: true,
            background: BackgroundSpec::LinearGradient {
                start: Rgba8::from_rgb(48, 28, 16),
                end: Rgba8::from_rgb(16, 11, 14),
                angle_deg: 145.0,
            },
            font_family: ".SystemUIFont".to_owned(),
            font_size_pt: 74.0,
            font_weight: 600,
            line_height: 1.35,
            text_color: Rgba8::from_rgb(255, 247, 232),
            h_align: HorizontalAlign::Center,
            v_align: VerticalAlign::Middle,
            stroke_width_px: 1.5,
            stroke_color: Rgba8::from_rgba(20, 10, 5, 160),
            shadow: Some(TextShadow {
                offset_x_px: 0.0,
                offset_y_px: 5.0,
                blur_radius_px: 12.0,
                color: Rgba8::from_rgba(0, 0, 0, 180),
            }),
            margins: SafeMargins::DEFAULT,
            footer_font_size_pt: 24.0,
            transition_ms: 300,
        },
        Theme {
            id: v5_theme("scripture-clean"),
            name: "Scripture Clean".to_owned(),
            target: ThemeTarget::Scripture,
            is_default: true,
            background: BackgroundSpec::Solid {
                color: Rgba8::from_rgb(15, 23, 34),
            },
            font_family: ".SystemUIFont".to_owned(),
            font_size_pt: 60.0,
            font_weight: 500,
            line_height: 1.42,
            text_color: Rgba8::from_rgb(241, 245, 249),
            h_align: HorizontalAlign::Left,
            v_align: VerticalAlign::Middle,
            stroke_width_px: 0.0,
            stroke_color: Rgba8::TRANSPARENT,
            shadow: Some(TextShadow {
                offset_x_px: 0.0,
                offset_y_px: 4.0,
                blur_radius_px: 10.0,
                color: Rgba8::from_rgba(0, 0, 0, 140),
            }),
            margins: SafeMargins {
                top_pct: 0.08,
                right_pct: 0.09,
                bottom_pct: 0.10,
                left_pct: 0.09,
            },
            footer_font_size_pt: 28.0,
            transition_ms: 220,
        },
    ]
}

/// Public-domain sample hymn 1 for out-of-the-box testing.
pub fn sample_amazing_grace() -> Song {
    let v1 = v5_section("grace-v1");
    let v2 = v5_section("grace-v2");
    let v3 = v5_section("grace-v3");
    let v4 = v5_section("grace-v4");
    let default_arr = v5_arrangement("grace-default");
    let short_arr = v5_arrangement("grace-short");

    Song {
        id: v5_song("amazing-grace"),
        title: "Amazing Grace".to_owned(),
        alternate_title: "Amazing Grace (My Chains Are Gone)".to_owned(),
        author: "John Newton".to_owned(),
        copyright: "Public Domain".to_owned(),
        ccli_number: "22025".to_owned(),
        default_theme_id: Some(v5_theme("sanctuary-dark")),
        default_arrangement_id: Some(default_arr),
        default_reflow_mode: ReflowMode::ByStanza,
        sections: vec![
            SongSection {
                id: v1,
                kind: SectionKind::Verse,
                label: "Verse 1".to_owned(),
                ordinal: 0,
                body: "Amazing grace! How sweet the sound\nThat saved a wretch like me!\nI once was lost, but now am found;\nWas blind, but now I see.".to_owned(),
            },
            SongSection {
                id: v2,
                kind: SectionKind::Verse,
                label: "Verse 2".to_owned(),
                ordinal: 1,
                body: "'Twas grace that taught my heart to fear,\nAnd grace my fears relieved;\nHow precious did that grace appear\nThe hour I first believed.".to_owned(),
            },
            SongSection {
                id: v3,
                kind: SectionKind::Verse,
                label: "Verse 3".to_owned(),
                ordinal: 2,
                body: "Through many dangers, toils, and snares,\nI have already come;\n'Tis grace hath brought me safe thus far,\nAnd grace will lead me home.".to_owned(),
            },
            SongSection {
                id: v4,
                kind: SectionKind::Verse,
                label: "Verse 4".to_owned(),
                ordinal: 3,
                body: "When we've been there ten thousand years,\nBright shining as the sun,\nWe've no less days to sing God's praise\nThan when we'd first begun.".to_owned(),
            },
        ],
        arrangements: vec![
            Arrangement {
                id: default_arr,
                name: "Default".to_owned(),
                section_ids: vec![v1, v2, v3, v4],
            },
            Arrangement {
                id: short_arr,
                name: "Short (2 Verses)".to_owned(),
                section_ids: vec![v1, v4],
            },
        ],
    }
}

/// Public-domain sample hymn 2 for out-of-the-box testing.
pub fn sample_come_thou_fount() -> Song {
    let v1 = v5_section("fount-v1");
    let v2 = v5_section("fount-v2");
    let v3 = v5_section("fount-v3");
    let default_arr = v5_arrangement("fount-default");

    Song {
        id: v5_song("come-thou-fount"),
        title: "Come, Thou Fount of Every Blessing".to_owned(),
        alternate_title: String::new(),
        author: "Robert Robinson".to_owned(),
        copyright: "Public Domain".to_owned(),
        ccli_number: "108389".to_owned(),
        default_theme_id: Some(v5_theme("sanctuary-dark")),
        default_arrangement_id: Some(default_arr),
        default_reflow_mode: ReflowMode::ByStanza,
        sections: vec![
            SongSection {
                id: v1,
                kind: SectionKind::Verse,
                label: "Verse 1".to_owned(),
                ordinal: 0,
                body: "Come, Thou Fount of every blessing,\nTune my heart to sing Thy grace;\nStreams of mercy, never ceasing,\nCall for songs of loudest praise.".to_owned(),
            },
            SongSection {
                id: v2,
                kind: SectionKind::Verse,
                label: "Verse 2".to_owned(),
                ordinal: 1,
                body: "Here I raise my Ebenezer;\nHither by Thy help I'm come;\nAnd I hope, by Thy good pleasure,\nSafely to arrive at home.".to_owned(),
            },
            SongSection {
                id: v3,
                kind: SectionKind::Verse,
                label: "Verse 3".to_owned(),
                ordinal: 2,
                body: "O to grace how great a debtor\nDaily I'm constrained to be!\nLet Thy goodness, like a fetter,\nBind my wandering heart to Thee.".to_owned(),
            },
        ],
        arrangements: vec![Arrangement {
            id: default_arr,
            name: "Default".to_owned(),
            section_ids: vec![v1, v2, v3],
        }],
    }
}

/// Public-domain sample hymn 3 with Chorus repeat to exercise non-destructive arrangements.
pub fn sample_it_is_well() -> Song {
    let v1 = v5_section("well-v1");
    let ch = v5_section("well-ch");
    let v2 = v5_section("well-v2");
    let default_arr = v5_arrangement("well-default");

    Song {
        id: v5_song("it-is-well"),
        title: "It Is Well with My Soul".to_owned(),
        alternate_title: "When Peace Like a River".to_owned(),
        author: "Horatio G. Spafford".to_owned(),
        copyright: "Public Domain".to_owned(),
        ccli_number: "25376".to_owned(),
        default_theme_id: Some(v5_theme("warm-amber-glow")),
        default_arrangement_id: Some(default_arr),
        default_reflow_mode: ReflowMode::ByStanza,
        sections: vec![
            SongSection {
                id: v1,
                kind: SectionKind::Verse,
                label: "Verse 1".to_owned(),
                ordinal: 0,
                body: "When peace, like a river, attendeth my way,\nWhen sorrows like sea billows roll;\nWhatever my lot, Thou hast taught me to say,\nIt is well, it is well with my soul.".to_owned(),
            },
            SongSection {
                id: ch,
                kind: SectionKind::Chorus,
                label: "Chorus".to_owned(),
                ordinal: 1,
                body: "It is well with my soul,\nIt is well, it is well with my soul.".to_owned(),
            },
            SongSection {
                id: v2,
                kind: SectionKind::Verse,
                label: "Verse 2".to_owned(),
                ordinal: 2,
                body: "My sin—oh, the bliss of this glorious thought!—\nMy sin, not in part but the whole,\nIs nailed to the cross, and I bear it no more,\nPraise the Lord, praise the Lord, O my soul!".to_owned(),
            },
        ],
        arrangements: vec![Arrangement {
            id: default_arr,
            name: "Default".to_owned(),
            section_ids: vec![v1, ch, v2, ch],
        }],
    }
}

pub fn default_sample_service() -> ServiceSchedule {
    let grace = sample_amazing_grace();
    let fount = sample_come_thou_fount();
    let well = sample_it_is_well();

    ServiceSchedule {
        id: ServiceId::from_v5(&SUNDAYDECK_NAMESPACE, b"seed:service:sunday-morning"),
        title: "Sunday Morning Worship".to_owned(),
        service_date: Some("2026-11-22".to_owned()),
        notes: "Default starter service schedule.".to_owned(),
        items: vec![
            ServiceItem {
                id: ServiceItemId::from_v5(&SUNDAYDECK_NAMESPACE, b"seed:item:welcome"),
                title: "Welcome & Announcements".to_owned(),
                theme_override_id: Some(v5_theme("warm-amber-glow")),
                payload: ServiceItemPayload::CustomSlide {
                    body: "Welcome Home\nWe're glad you're worshipping with us today.".to_owned(),
                    footer: Some("Service begins at 10:00 AM  •  Kids check-in in West Hall".to_owned()),
                },
            },
            ServiceItem {
                id: ServiceItemId::from_v5(&SUNDAYDECK_NAMESPACE, b"seed:item:grace"),
                title: grace.title.clone(),
                theme_override_id: None,
                payload: ServiceItemPayload::Song {
                    song_id: grace.id,
                    arrangement_id: grace.default_arrangement_id,
                    reflow_override: None,
                },
            },
            ServiceItem {
                id: ServiceItemId::from_v5(&SUNDAYDECK_NAMESPACE, b"seed:item:fount"),
                title: fount.title.clone(),
                theme_override_id: None,
                payload: ServiceItemPayload::Song {
                    song_id: fount.id,
                    arrangement_id: fount.default_arrangement_id,
                    reflow_override: None,
                },
            },
            ServiceItem {
                id: ServiceItemId::from_v5(&SUNDAYDECK_NAMESPACE, b"seed:item:scripture"),
                title: "Psalm 23:1-3 (WEB)".to_owned(),
                theme_override_id: Some(v5_theme("scripture-clean")),
                payload: ServiceItemPayload::Scripture {
                    translation_code: "WEB".to_owned(),
                    reference: "Psalm 23:1-3".to_owned(),
                    cached_verses: vec![
                        (
                            "1".to_owned(),
                            "Yahweh is my shepherd; I shall lack nothing.".to_owned(),
                        ),
                        (
                            "2".to_owned(),
                            "He makes me lie down in green pastures. He leads me beside still waters.".to_owned(),
                        ),
                        (
                            "3".to_owned(),
                            "He restores my soul. He guides me in the paths of righteousness for his name's sake.".to_owned(),
                        ),
                    ],
                },
            },
            ServiceItem {
                id: ServiceItemId::from_v5(&SUNDAYDECK_NAMESPACE, b"seed:item:well"),
                title: well.title.clone(),
                theme_override_id: None,
                payload: ServiceItemPayload::Song {
                    song_id: well.id,
                    arrangement_id: well.default_arrangement_id,
                    reflow_override: None,
                },
            },
        ],
    }
}
