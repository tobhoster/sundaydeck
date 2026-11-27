use crate::bible_import::{ParsedBible, ParsedVerse};

fn verse(book_number: u8, chapter: u16, verse: u16, text: &str) -> ParsedVerse {
    ParsedVerse {
        book_number,
        chapter,
        verse,
        text: text.to_owned(),
    }
}

/// Built-in Berean Standard Bible (BSB, CC0 Public Domain) starter passages available offline
/// immediately on first launch before background full-Bible sync finishes.
pub(crate) fn starter_bsb() -> ParsedBible {
    const VERSES: &[(u8, u16, u16, &str)] = &[
        (
            1,
            1,
            1,
            "In the beginning God created the heavens and the earth.",
        ),
        (
            1,
            1,
            2,
            "Now the earth was formless and void, and darkness was over the surface of the deep. And the Spirit of God was hovering over the surface of the waters.",
        ),
        (
            1,
            1,
            3,
            "And God said, \"Let there be light,\" and there was light.",
        ),
        (19, 23, 1, "The LORD is my shepherd; I shall not want."),
        (
            19,
            23,
            2,
            "He makes me lie down in green pastures; He leads me beside quiet waters.",
        ),
        (
            19,
            23,
            3,
            "He restores my soul; He guides me in the paths of righteousness for the sake of His name.",
        ),
        (
            19,
            23,
            4,
            "Even though I walk through the valley of the shadow of death, I will fear no evil, for You are with me; Your rod and Your staff, they comfort me.",
        ),
        (
            19,
            23,
            5,
            "You prepare a table before me in the presence of my enemies. You anoint my head with oil; my cup overflows.",
        ),
        (
            19,
            23,
            6,
            "Surely goodness and mercy will follow me all the days of my life, and I will dwell in the house of the LORD forever.",
        ),
        (
            19,
            91,
            1,
            "He who dwells in the shelter of the Most High will abide in the shadow of the Almighty.",
        ),
        (
            19,
            91,
            2,
            "I will say to the LORD, \"You are my refuge and my fortress, my God, in whom I trust.\"",
        ),
        (
            19,
            100,
            1,
            "Make a joyful noise to the LORD, all the earth.",
        ),
        (
            19,
            100,
            2,
            "Serve the LORD with gladness; come into His presence with joyful songs.",
        ),
        (
            19,
            100,
            4,
            "Enter His gates with thanksgiving and His courts with praise; give thanks to Him and bless His name.",
        ),
        (
            19,
            100,
            5,
            "For the LORD is good, and His loving devotion endures forever; His faithfulness continues to all generations.",
        ),
        (
            20,
            3,
            5,
            "Trust in the LORD with all your heart, and lean not on your own understanding;",
        ),
        (
            20,
            3,
            6,
            "in all your ways acknowledge Him, and He will make your paths straight.",
        ),
        (
            23,
            40,
            31,
            "But those who wait upon the LORD will renew their strength; they will mount up with wings like eagles; they will run and not grow weary, they will walk and not faint.",
        ),
        (
            40,
            5,
            3,
            "Blessed are the poor in spirit, for theirs is the kingdom of heaven.",
        ),
        (
            40,
            5,
            14,
            "You are the light of the world. A city on a hill cannot be hidden.",
        ),
        (
            40,
            28,
            19,
            "Therefore go and make disciples of all nations, baptizing them in the name of the Father, and of the Son, and of the Holy Spirit,",
        ),
        (
            43,
            3,
            16,
            "For God so loved the world that He gave His one and only Son, that everyone who believes in Him shall not perish but have eternal life.",
        ),
        (
            43,
            3,
            17,
            "For God did not send His Son into the world to condemn the world, but to save the world through Him.",
        ),
        (
            45,
            8,
            28,
            "And we know that God works all things together for the good of those who love Him, who are called according to His purpose.",
        ),
        (
            45,
            8,
            38,
            "For I am convinced that neither death nor life, neither angels nor principalities, neither the present nor the future, nor any powers,",
        ),
        (
            45,
            8,
            39,
            "neither height nor depth, nor anything else in all creation, will be able to separate us from the love of God that is in Christ Jesus our Lord.",
        ),
        (
            46,
            13,
            4,
            "Love is patient, love is kind. It does not envy, it does not boast, it is not proud.",
        ),
        (
            46,
            13,
            7,
            "It bears all things, believes all things, hopes all things, endures all things.",
        ),
        (46, 13, 8, "Love never fails."),
        (
            50,
            4,
            6,
            "Be anxious for nothing, but in everything, by prayer and petition, with thanksgiving, present your requests to God.",
        ),
        (
            50,
            4,
            7,
            "And the peace of God, which surpasses all understanding, will guard your hearts and your minds in Christ Jesus.",
        ),
        (
            66,
            21,
            4,
            "He will wipe away every tear from their eyes, and there will be no more death or mourning or crying or pain, for the former things have passed away.",
        ),
    ];

    ParsedBible {
        code: "BSB".to_owned(),
        name: "Berean Standard Bible (Starter)".to_owned(),
        language: "en".to_owned(),
        copyright: "Public Domain (CC0)".to_owned(),
        verses: VERSES
            .iter()
            .map(|(b, c, v, t)| verse(*b, *c, *v, t))
            .collect(),
    }
}
