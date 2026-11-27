use crate::defaults::{sample_amazing_grace, sample_come_thou_fount, sample_it_is_well};
use crate::global_content::HymnalEntry;
use crate::ids::{ArrangementId, SUNDAYDECK_NAMESPACE, SectionId, SongId, ThemeId};
use crate::song::{Arrangement, ReflowMode, SectionKind, Song, SongSection};
use crate::starter_hymns_more::more_public_domain_hymns;

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

pub(crate) struct HymnSeed<'a> {
    pub slug: &'a str,
    pub title: &'a str,
    pub alt: &'a str,
    pub author: &'a str,
    pub primary_lang: &'a str,
    pub secondary_lang: Option<&'a str>,
    pub theme_tag: &'a str,
    pub hymnals: &'a [(&'a str, &'a str, u32)],
    pub traditions: &'a [&'a str],
    pub stanzas: &'a [(SectionKind, &'a str, &'a str, Option<&'a str>)],
    pub order: &'a [usize],
}

pub(crate) fn build_hymn(seed: HymnSeed<'_>) -> Song {
    let arr_id = v5_arrangement(&format!("{}-default", seed.slug));
    let sections: Vec<SongSection> = seed
        .stanzas
        .iter()
        .enumerate()
        .map(|(ordinal, (kind, label, body, sec_body))| SongSection {
            id: v5_section(&format!("{}-s{ordinal}", seed.slug)),
            kind: *kind,
            label: (*label).to_owned(),
            ordinal: u32::try_from(ordinal).unwrap_or(0),
            body: (*body).to_owned(),
            secondary_body: sec_body.map(str::to_owned),
            stage_chords: None,
        })
        .collect();

    let section_ids: Vec<SectionId> = if seed.order.is_empty() {
        sections.iter().map(|s| s.id).collect()
    } else {
        seed.order
            .iter()
            .filter_map(|&idx| sections.get(idx).map(|s| s.id))
            .collect()
    };

    Song {
        id: v5_song(seed.slug),
        title: seed.title.to_owned(),
        alternate_title: seed.alt.to_owned(),
        author: seed.author.to_owned(),
        copyright: "Public Domain".to_owned(),
        ccli_number: String::new(),
        default_theme_id: Some(v5_theme(seed.theme_tag)),
        default_arrangement_id: Some(arr_id),
        default_reflow_mode: ReflowMode::ByStanza,
        primary_lang: seed.primary_lang.to_owned(),
        secondary_lang: seed.secondary_lang.map(str::to_owned),
        hymnal_entries: seed
            .hymnals
            .iter()
            .map(|(code, name, num)| HymnalEntry {
                hymnal_code: (*code).to_owned(),
                hymnal_name: (*name).to_owned(),
                hymn_number: *num,
            })
            .collect(),
        traditions: seed.traditions.iter().map(|t| (*t).to_owned()).collect(),
        sections,
        arrangements: vec![Arrangement {
            id: arr_id,
            name: "Default".to_owned(),
            section_ids,
        }],
    }
}

/// Verified 100% Public Domain starter hymnal (all pre-1920 authors/translations) bundled offline
/// so SundayDeck opens populated and ready for worship even without an internet connection.
pub fn starter_songs() -> Vec<Song> {
    let mut songs = vec![
        sample_amazing_grace(),
        sample_come_thou_fount(),
        sample_it_is_well(),
    ];
    songs.extend(core_public_domain_hymns());
    songs.extend(more_public_domain_hymns());
    songs
}

fn core_public_domain_hymns() -> Vec<Song> {
    use SectionKind::{Chorus, Verse};
    vec![
        build_hymn(HymnSeed {
            slug: "holy-holy-holy",
            title: "Holy, Holy, Holy! Lord God Almighty",
            alt: "Nicaea",
            author: "Reginald Heber (1826)",
            primary_lang: "en",
            secondary_lang: None,
            theme_tag: "sanctuary-mountains",
            hymnals: &[
                ("BH", "Baptist Hymnal", 1),
                ("UMH", "United Methodist Hymnal", 64),
            ],
            traditions: &["Traditional Hymns", "Baptist", "Methodist"],
            stanzas: &[
                (
                    Verse,
                    "Verse 1",
                    "Holy, holy, holy! Lord God Almighty!\nEarly in the morning our song shall rise to Thee;\nHoly, holy, holy! Merciful and mighty!\nGod in three Persons, blessed Trinity!",
                    None,
                ),
                (
                    Verse,
                    "Verse 2",
                    "Holy, holy, holy! All the saints adore Thee,\nCasting down their golden crowns around the glassy sea;\nCherubim and seraphim falling down before Thee,\nWho wert, and art, and evermore shalt be.",
                    None,
                ),
                (
                    Verse,
                    "Verse 3",
                    "Holy, holy, holy! Though the darkness hide Thee,\nThough the eye of sinful man Thy glory may not see;\nOnly Thou art holy; there is none beside Thee,\nPerfect in power, in love, and purity.",
                    None,
                ),
                (
                    Verse,
                    "Verse 4",
                    "Holy, holy, holy! Lord God Almighty!\nAll Thy works shall praise Thy Name, in earth, and sky, and sea;\nHoly, holy, holy! Merciful and mighty!\nGod in three Persons, blessed Trinity!",
                    None,
                ),
            ],
            order: &[],
        }),
        build_hymn(HymnSeed {
            slug: "o-for-a-thousand-tongues",
            title: "O for a Thousand Tongues to Sing",
            alt: "Azmon",
            author: "Charles Wesley (1739)",
            primary_lang: "en",
            secondary_lang: None,
            theme_tag: "golden-light-motion",
            hymnals: &[
                ("UMH", "United Methodist Hymnal", 57),
                ("BH", "Baptist Hymnal", 216),
            ],
            traditions: &["Traditional Hymns", "Methodist", "Baptist"],
            stanzas: &[
                (
                    Verse,
                    "Verse 1",
                    "O for a thousand tongues to sing\nMy great Redeemer's praise,\nThe glories of my God and King,\nThe triumphs of His grace!",
                    None,
                ),
                (
                    Verse,
                    "Verse 2",
                    "My gracious Master and my God,\nAssist me to proclaim,\nTo spread through all the earth abroad\nThe honors of Thy name.",
                    None,
                ),
                (
                    Verse,
                    "Verse 3",
                    "Jesus! the name that charms our fears,\nThat bids our sorrows cease;\n'Tis music in the sinner's ears,\n'Tis life, and health, and peace.",
                    None,
                ),
                (
                    Verse,
                    "Verse 4",
                    "He breaks the power of canceled sin,\nHe sets the prisoner free;\nHis blood can make the foulest clean;\nHis blood availed for me.",
                    None,
                ),
            ],
            order: &[],
        }),
        build_hymn(HymnSeed {
            slug: "blessed-assurance",
            title: "Blessed Assurance",
            alt: "This Is My Story",
            author: "Fanny J. Crosby (1873)",
            primary_lang: "en",
            secondary_lang: None,
            theme_tag: "warm-amber-glow",
            hymnals: &[
                ("BH", "Baptist Hymnal", 334),
                ("UMH", "United Methodist Hymnal", 369),
            ],
            traditions: &["Traditional Hymns", "Baptist", "Methodist"],
            stanzas: &[
                (
                    Verse,
                    "Verse 1",
                    "Blessed assurance, Jesus is mine!\nO what a foretaste of glory divine!\nHeir of salvation, purchase of God,\nBorn of His Spirit, washed in His blood.",
                    None,
                ),
                (
                    Chorus,
                    "Chorus",
                    "This is my story, this is my song,\nPraising my Savior all the day long;\nThis is my story, this is my song,\nPraising my Savior all the day long.",
                    None,
                ),
                (
                    Verse,
                    "Verse 2",
                    "Perfect submission, perfect delight,\nVisions of rapture now burst on my sight;\nAngels descending bring from above\nEchoes of mercy, whispers of love.",
                    None,
                ),
                (
                    Verse,
                    "Verse 3",
                    "Perfect submission, all is at rest,\nI in my Savior am happy and blest,\nWatching and waiting, looking above,\nFilled with His goodness, lost in His love.",
                    None,
                ),
            ],
            order: &[0, 1, 2, 1, 3, 1],
        }),
        build_hymn(HymnSeed {
            slug: "be-thou-my-vision",
            title: "Be Thou My Vision",
            alt: "Slane",
            author: "Dallán Forgaill · trans. Eleanor Hull (1912)",
            primary_lang: "en",
            secondary_lang: None,
            theme_tag: "sanctuary-mountains",
            hymnals: &[
                ("UMH", "United Methodist Hymnal", 451),
                ("BH", "Baptist Hymnal", 60),
            ],
            traditions: &["Traditional Hymns", "Mainline"],
            stanzas: &[
                (
                    Verse,
                    "Verse 1",
                    "Be Thou my Vision, O Lord of my heart;\nNaught be all else to me, save that Thou art—\nThou my best thought, by day or by night,\nWaking or sleeping, Thy presence my light.",
                    None,
                ),
                (
                    Verse,
                    "Verse 2",
                    "Be Thou my Wisdom, and Thou my true Word;\nI ever with Thee and Thou with me, Lord;\nThou my great Father, I Thy true son;\nThou in me dwelling, and I with Thee one.",
                    None,
                ),
                (
                    Verse,
                    "Verse 3",
                    "Riches I heed not, nor man's empty praise,\nThou mine inheritance, now and always:\nThou and Thou only, first in my heart,\nHigh King of heaven, my Treasure Thou art.",
                    None,
                ),
                (
                    Verse,
                    "Verse 4",
                    "High King of heaven, my victory won,\nMay I reach heaven's joys, O bright heaven's Sun!\nHeart of my own heart, whatever befall,\nStill be my Vision, O Ruler of all.",
                    None,
                ),
            ],
            order: &[],
        }),
        build_hymn(HymnSeed {
            slug: "crown-him-with-many-crowns",
            title: "Crown Him with Many Crowns",
            alt: "Diademata",
            author: "Matthew Bridges (1851) · Godfrey Thring (1874)",
            primary_lang: "en",
            secondary_lang: None,
            theme_tag: "golden-light-motion",
            hymnals: &[
                ("BH", "Baptist Hymnal", 161),
                ("UMH", "United Methodist Hymnal", 327),
            ],
            traditions: &["Traditional Hymns", "Baptist", "Methodist"],
            stanzas: &[
                (
                    Verse,
                    "Verse 1",
                    "Crown Him with many crowns,\nThe Lamb upon His throne;\nHark! how the heavenly anthem drowns\nAll music but its own.",
                    None,
                ),
                (
                    Verse,
                    "Verse 2",
                    "Crown Him the Lord of love!\nBehold His hands and side,\nRich wounds, yet visible above,\nIn beauty glorified.",
                    None,
                ),
                (
                    Verse,
                    "Verse 3",
                    "Crown Him the Lord of life,\nWho triumphed o'er the grave,\nAnd rose victorious in the strife\nFor those He came to save.",
                    None,
                ),
                (
                    Verse,
                    "Verse 4",
                    "Crown Him the Lord of years,\nThe Potentate of time,\nCreator of the rolling spheres,\nIneffably sublime!",
                    None,
                ),
            ],
            order: &[],
        }),
        build_hymn(HymnSeed {
            slug: "a-mighty-fortress",
            title: "A Mighty Fortress Is Our God",
            alt: "Ein' Feste Burg",
            author: "Martin Luther (1529) · trans. Frederick Hedge (1852)",
            primary_lang: "en",
            secondary_lang: None,
            theme_tag: "sanctuary-dark",
            hymnals: &[
                ("UMH", "United Methodist Hymnal", 110),
                ("BH", "Baptist Hymnal", 8),
            ],
            traditions: &["Traditional Hymns", "Mainline", "Lutheran"],
            stanzas: &[
                (
                    Verse,
                    "Verse 1",
                    "A mighty fortress is our God,\nA bulwark never failing;\nOur helper He amid the flood\nOf mortal ills prevailing.",
                    None,
                ),
                (
                    Verse,
                    "Verse 2",
                    "Did we in our own strength confide,\nOur striving would be losing;\nWere not the right Man on our side,\nThe Man of God's own choosing.",
                    None,
                ),
                (
                    Verse,
                    "Verse 3",
                    "And though this world, with devils filled,\nShould threaten to undo us,\nWe will not fear, for God hath willed\nHis truth to triumph through us.",
                    None,
                ),
                (
                    Verse,
                    "Verse 4",
                    "That word above all earthly powers,\nNo thanks to them, abideth;\nThe Spirit and the gifts are ours\nThrough Him who with us sideth.",
                    None,
                ),
            ],
            order: &[],
        }),
        build_hymn(HymnSeed {
            slug: "joyful-joyful-we-adore-thee",
            title: "Joyful, Joyful, We Adore Thee",
            alt: "Hymn to Joy",
            author: "Henry van Dyke (1907)",
            primary_lang: "en",
            secondary_lang: None,
            theme_tag: "golden-light-motion",
            hymnals: &[
                ("UMH", "United Methodist Hymnal", 89),
                ("BH", "Baptist Hymnal", 13),
            ],
            traditions: &["Traditional Hymns", "Mainline"],
            stanzas: &[
                (
                    Verse,
                    "Verse 1",
                    "Joyful, joyful, we adore Thee,\nGod of glory, Lord of love;\nHearts unfold like flowers before Thee,\nOpening to the sun above.",
                    None,
                ),
                (
                    Verse,
                    "Verse 2",
                    "All Thy works with joy surround Thee,\nEarth and heaven reflect Thy rays,\nStars and angels sing around Thee,\nCenter of unbroken praise.",
                    None,
                ),
                (
                    Verse,
                    "Verse 3",
                    "Thou art giving and forgiving,\nEver blessing, ever blest,\nWellspring of the joy of living,\nOcean depth of happy rest!",
                    None,
                ),
                (
                    Verse,
                    "Verse 4",
                    "Mortals, join the happy chorus\nWhich the morning stars began;\nFather love is reigning o'er us,\nBrother love binds man to man.",
                    None,
                ),
            ],
            order: &[],
        }),
    ]
}
