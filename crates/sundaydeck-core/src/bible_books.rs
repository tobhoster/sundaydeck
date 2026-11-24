use crate::bible_aliases::{DEUTEROCANON_BOOKS, lookup_localized_alias, strip_diacritics};

/// Metadata for one canonical book of the Bible (66 Protestant + 7 Catholic Deuterocanon + ESG/DAG).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BibleBookMeta {
    pub number: u8,
    pub name: &'static str,
    pub osis_id: &'static str,
    pub usfx_id: &'static str,
    pub single_chapter: bool,
    pub is_deuterocanon: bool,
    pub aliases: &'static [&'static str],
}

macro_rules! book {
    ($num:expr, $name:expr, $osis:expr, $usfx:expr, $single:expr, [$($alias:expr),* $(,)?]) => {
        BibleBookMeta {
            number: $num,
            name: $name,
            osis_id: $osis,
            usfx_id: $usfx,
            single_chapter: $single,
            is_deuterocanon: false,
            aliases: &[$($alias),*],
        }
    };
}

/// Canonical 66-book Protestant order, acting as a single compiled source of truth
/// for the smart reference parser and OSIS/USFX/Zefania XML importers.
pub const CANONICAL_BOOKS: &[BibleBookMeta] = &[
    book!(1, "Genesis", "Gen", "GEN", false, ["gen", "ge", "gn"]),
    book!(2, "Exodus", "Exod", "EXO", false, ["exod", "exo", "ex"]),
    book!(3, "Leviticus", "Lev", "LEV", false, ["lev", "le", "lv"]),
    book!(4, "Numbers", "Num", "NUM", false, ["num", "nu", "nm", "nb"]),
    book!(
        5,
        "Deuteronomy",
        "Deut",
        "DEU",
        false,
        ["deut", "deu", "dt"]
    ),
    book!(6, "Joshua", "Josh", "JOS", false, ["josh", "jos", "jsh"]),
    book!(
        7,
        "Judges",
        "Judg",
        "JDG",
        false,
        ["judg", "jdg", "jg", "jdgs"]
    ),
    book!(
        8,
        "Ruth",
        "Ruth",
        "RUT",
        false,
        ["ruth", "rut", "rth", "ru"]
    ),
    book!(
        9,
        "1 Samuel",
        "1Sam",
        "1SA",
        false,
        ["1sam", "1sa", "1sm", "isam", "1samuel"]
    ),
    book!(
        10,
        "2 Samuel",
        "2Sam",
        "2SA",
        false,
        ["2sam", "2sa", "2sm", "iisam", "2samuel"]
    ),
    book!(
        11,
        "1 Kings",
        "1Kgs",
        "1KI",
        false,
        ["1kgs", "1ki", "1kg", "ikgs", "1kings"]
    ),
    book!(
        12,
        "2 Kings",
        "2Kgs",
        "2KI",
        false,
        ["2kgs", "2ki", "2kg", "iikgs", "2kings"]
    ),
    book!(
        13,
        "1 Chronicles",
        "1Chr",
        "1CH",
        false,
        ["1chr", "1ch", "1chron", "1chronicles"]
    ),
    book!(
        14,
        "2 Chronicles",
        "2Chr",
        "2CH",
        false,
        ["2chr", "2ch", "2chron", "2chronicles"]
    ),
    book!(15, "Ezra", "Ezra", "EZR", false, ["ezra", "ezr"]),
    book!(16, "Nehemiah", "Neh", "NEH", false, ["neh", "ne"]),
    book!(17, "Esther", "Esth", "EST", false, ["esth", "est", "es"]),
    book!(18, "Job", "Job", "JOB", false, ["job", "jb"]),
    book!(
        19,
        "Psalms",
        "Ps",
        "PSA",
        false,
        ["ps", "psa", "psalm", "psalms", "psm", "pss"]
    ),
    book!(
        20,
        "Proverbs",
        "Prov",
        "PRO",
        false,
        ["prov", "pro", "prv", "pr"]
    ),
    book!(
        21,
        "Ecclesiastes",
        "Eccl",
        "ECC",
        false,
        ["eccl", "ecc", "eccles", "ec", "qoh"]
    ),
    book!(
        22,
        "Song of Solomon",
        "Song",
        "SNG",
        false,
        ["song", "sos", "sng", "cant", "songofsolomon", "songofsongs"]
    ),
    book!(23, "Isaiah", "Isa", "ISA", false, ["isa", "is"]),
    book!(24, "Jeremiah", "Jer", "JER", false, ["jer", "je", "jr"]),
    book!(25, "Lamentations", "Lam", "LAM", false, ["lam", "la"]),
    book!(26, "Ezekiel", "Ezek", "EZK", false, ["ezek", "ezk", "eze"]),
    book!(27, "Daniel", "Dan", "DAN", false, ["dan", "da", "dn"]),
    book!(28, "Hosea", "Hos", "HOS", false, ["hos", "ho"]),
    book!(29, "Joel", "Joel", "JOL", false, ["joel", "jol", "jl"]),
    book!(30, "Amos", "Amos", "AMO", false, ["amos", "amo", "am"]),
    book!(31, "Obadiah", "Obad", "OBA", true, ["obad", "oba", "ob"]),
    book!(32, "Jonah", "Jonah", "JON", false, ["jonah", "jon", "jnh"]),
    book!(33, "Micah", "Mic", "MIC", false, ["mic", "mc"]),
    book!(34, "Nahum", "Nah", "NAM", false, ["nah", "nam", "na"]),
    book!(35, "Habakkuk", "Hab", "HAB", false, ["hab", "hb"]),
    book!(36, "Zephaniah", "Zeph", "ZEP", false, ["zeph", "zep", "zp"]),
    book!(37, "Haggai", "Hag", "HAG", false, ["hag", "hg"]),
    book!(38, "Zechariah", "Zech", "ZEC", false, ["zech", "zec", "zc"]),
    book!(39, "Malachi", "Mal", "MAL", false, ["mal", "ml"]),
    book!(40, "Matthew", "Matt", "MAT", false, ["matt", "mat", "mt"]),
    book!(
        41,
        "Mark",
        "Mark",
        "MRK",
        false,
        ["mark", "mrk", "mk", "mr"]
    ),
    book!(42, "Luke", "Luke", "LUK", false, ["luke", "luk", "lk"]),
    book!(
        43,
        "John",
        "John",
        "JHN",
        false,
        ["john", "jhn", "jn", "joh"]
    ),
    book!(44, "Acts", "Acts", "ACT", false, ["acts", "act", "ac"]),
    book!(45, "Romans", "Rom", "ROM", false, ["rom", "ro", "rm"]),
    book!(
        46,
        "1 Corinthians",
        "1Cor",
        "1CO",
        false,
        ["1cor", "1co", "icor", "1corinthians"]
    ),
    book!(
        47,
        "2 Corinthians",
        "2Cor",
        "2CO",
        false,
        ["2cor", "2co", "iicor", "2corinthians"]
    ),
    book!(48, "Galatians", "Gal", "GAL", false, ["gal", "ga"]),
    book!(49, "Ephesians", "Eph", "EPH", false, ["eph", "ephes"]),
    book!(
        50,
        "Philippians",
        "Phil",
        "PHP",
        false,
        ["phil", "php", "pp"]
    ),
    book!(51, "Colossians", "Col", "COL", false, ["col", "co"]),
    book!(
        52,
        "1 Thessalonians",
        "1Thess",
        "1TH",
        false,
        ["1thess", "1th", "1thes", "1thessalonians"]
    ),
    book!(
        53,
        "2 Thessalonians",
        "2Thess",
        "2TH",
        false,
        ["2thess", "2th", "2thes", "2thessalonians"]
    ),
    book!(
        54,
        "1 Timothy",
        "1Tim",
        "1TI",
        false,
        ["1tim", "1ti", "1tm", "1timothy"]
    ),
    book!(
        55,
        "2 Timothy",
        "2Tim",
        "2TI",
        false,
        ["2tim", "2ti", "2tm", "2timothy"]
    ),
    book!(56, "Titus", "Titus", "TIT", false, ["titus", "tit", "ti"]),
    book!(
        57,
        "Philemon",
        "Phlm",
        "PHM",
        true,
        ["phlm", "phm", "philem", "pm"]
    ),
    book!(58, "Hebrews", "Heb", "HEB", false, ["heb", "he"]),
    book!(
        59,
        "James",
        "Jas",
        "JAS",
        false,
        ["jas", "jm", "jam", "james"]
    ),
    book!(
        60,
        "1 Peter",
        "1Pet",
        "1PE",
        false,
        ["1pet", "1pe", "1pt", "1peter"]
    ),
    book!(
        61,
        "2 Peter",
        "2Pet",
        "2PE",
        false,
        ["2pet", "2pe", "2pt", "2peter"]
    ),
    book!(
        62,
        "1 John",
        "1John",
        "1JN",
        false,
        ["1john", "1jn", "1jhn", "1jo"]
    ),
    book!(
        63,
        "2 John",
        "2John",
        "2JN",
        true,
        ["2john", "2jn", "2jhn", "2jo"]
    ),
    book!(
        64,
        "3 John",
        "3John",
        "3JN",
        true,
        ["3john", "3jn", "3jhn", "3jo"]
    ),
    book!(65, "Jude", "Jude", "JUD", true, ["jude", "jud", "jd"]),
    book!(
        66,
        "Revelation",
        "Rev",
        "REV",
        false,
        ["rev", "re", "rv", "revelations"]
    ),
];

/// Iterates across all 75 supported USFM canonical books (66 Protestant + 7 Catholic Deuterocanon + ESG/DAG).
pub fn all_canonical_books() -> impl Iterator<Item = &'static BibleBookMeta> {
    CANONICAL_BOOKS.iter().chain(DEUTEROCANON_BOOKS.iter())
}

pub fn book_by_number(number: u8) -> Option<&'static BibleBookMeta> {
    all_canonical_books().find(|b| b.number == number)
}

pub fn book_by_osis(osis_id: &str) -> Option<&'static BibleBookMeta> {
    all_canonical_books()
        .find(|b| b.osis_id.eq_ignore_ascii_case(osis_id))
        .or_else(|| match_book_name(osis_id))
}

pub fn book_by_usfx(usfx_id: &str) -> Option<&'static BibleBookMeta> {
    all_canonical_books()
        .find(|b| b.usfx_id.eq_ignore_ascii_case(usfx_id))
        .or_else(|| book_by_osis(usfx_id))
}

/// Resolves a user-typed book name or shorthand (`jn`, `1cor`, `wis`, `sir`, `sab`, `sal`, `joao`)
/// across English, Spanish, Portuguese, and French aliases with diacritic folding.
pub fn match_book_name(raw: &str) -> Option<&'static BibleBookMeta> {
    let norm = normalize_book_token(raw);
    if norm.is_empty() {
        return None;
    }

    // 1. Exact full name, USFM code, OSIS ID, or canonical alias match
    for meta in all_canonical_books() {
        if normalize_book_token(meta.name) == norm
            || meta.usfx_id.eq_ignore_ascii_case(&norm)
            || meta.osis_id.eq_ignore_ascii_case(&norm)
            || meta.aliases.iter().any(|alias| *alias == norm)
        {
            return Some(meta);
        }
    }

    // 2. Exact localized alias match (Spanish, Portuguese, French)
    if let Some(book_num) = lookup_localized_alias(&norm) {
        return book_by_number(book_num);
    }

    // 3. Prefix match on full normalized name or alias
    all_canonical_books().find(|meta| {
        normalize_book_token(meta.name).starts_with(&norm)
            || meta.aliases.iter().any(|alias| alias.starts_with(&norm))
    })
}

/// Returns up to `limit` matching Bible books for shorthand autocomplete.
pub fn autocomplete_books(prefix: &str, limit: usize) -> Vec<&'static BibleBookMeta> {
    let norm = normalize_book_token(prefix);
    if norm.is_empty() {
        return all_canonical_books().take(limit).collect();
    }

    let mut matches: Vec<&'static BibleBookMeta> = all_canonical_books()
        .filter(|meta| {
            normalize_book_token(meta.name).starts_with(&norm)
                || meta.aliases.iter().any(|alias| alias.starts_with(&norm))
        })
        .take(limit)
        .collect();

    if matches.is_empty()
        && let Some(num) = lookup_localized_alias(&norm)
        && let Some(meta) = book_by_number(num)
    {
        matches.push(meta);
    }

    if matches.is_empty() {
        matches = all_canonical_books()
            .filter(|meta| normalize_book_token(meta.name).contains(&norm))
            .take(limit)
            .collect();
    }
    matches
}

pub(crate) fn normalize_book_token(raw: &str) -> String {
    let trimmed = strip_diacritics(raw.trim());
    let replaced = if let Some(rest) = trimmed.strip_prefix("iii ") {
        format!("3{rest}")
    } else if let Some(rest) = trimmed.strip_prefix("ii ") {
        format!("2{rest}")
    } else if let Some(rest) = trimmed.strip_prefix("i ") {
        format!("1{rest}")
    } else if let Some(rest) = trimmed.strip_prefix("first ") {
        format!("1{rest}")
    } else if let Some(rest) = trimmed.strip_prefix("second ") {
        format!("2{rest}")
    } else if let Some(rest) = trimmed.strip_prefix("third ") {
        format!("3{rest}")
    } else {
        trimmed
    };

    replaced
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}
