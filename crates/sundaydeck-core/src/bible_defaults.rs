use crate::bible_defaults_bsb::starter_bsb;
use crate::bible_import::{ParsedBible, ParsedVerse};

macro_rules! v {
    ($book:expr, $ch:expr, $vs:expr, $txt:expr) => {
        ParsedVerse {
            book_number: $book,
            chapter: $ch,
            verse: $vs,
            text: $txt.to_owned(),
        }
    };
}

/// Built-in offline public-domain starter passages in Berean Standard Bible (BSB, CC0),
/// World English Bible (WEB), and KJV so multi-translation browsing, FTS5 search,
/// smart references, and pagination work immediately even offline.
pub fn starter_bibles() -> Vec<ParsedBible> {
    vec![starter_bsb(), starter_web(), starter_kjv()]
}

fn starter_web() -> ParsedBible {
    ParsedBible {
        code: "WEB".to_owned(),
        name: "World English Bible (Starter)".to_owned(),
        language: "en".to_owned(),
        copyright: "Public Domain".to_owned(),
        verses: vec![
            v!(
                1,
                1,
                1,
                "In the beginning, God created the heavens and the earth."
            ),
            v!(
                1,
                1,
                2,
                "The earth was formless and empty. Darkness was on the surface of the deep and God's Spirit was hovering over the surface of the waters."
            ),
            v!(
                1,
                1,
                3,
                "God said, \"Let there be light,\" and there was light."
            ),
            v!(19, 23, 1, "Yahweh is my shepherd; I shall lack nothing."),
            v!(
                19,
                23,
                2,
                "He makes me lie down in green pastures. He leads me beside still waters."
            ),
            v!(
                19,
                23,
                3,
                "He restores my soul. He guides me in the paths of righteousness for his name's sake."
            ),
            v!(
                19,
                23,
                4,
                "Even though I walk through the valley of the shadow of death, I will fear no evil, for you are with me. Your rod and your staff, they comfort me."
            ),
            v!(
                19,
                23,
                5,
                "You prepare a table before me in the presence of my enemies. You anoint my head with oil. My cup runs over."
            ),
            v!(
                19,
                23,
                6,
                "Surely goodness and loving kindness shall follow me all the days of my life, and I will dwell in Yahweh's house forever."
            ),
            v!(19, 100, 1, "Shout for joy to Yahweh, all you lands!"),
            v!(
                19,
                100,
                2,
                "Serve Yahweh with gladness. Come before his presence with singing."
            ),
            v!(
                19,
                100,
                3,
                "Know that Yahweh, he is God. It is he who has made us, and we are his. We are his people, and the sheep of his pasture."
            ),
            v!(
                19,
                100,
                4,
                "Enter into his gates with thanksgiving, and into his courts with praise. Give thanks to him, and bless his name."
            ),
            v!(
                19,
                100,
                5,
                "For Yahweh is good. His loving kindness endures forever, his faithfulness to all generations."
            ),
            v!(
                20,
                3,
                5,
                "Trust in Yahweh with all your heart, and don't lean on your own understanding."
            ),
            v!(
                20,
                3,
                6,
                "In all your ways acknowledge him, and he will make your paths straight."
            ),
            v!(
                23,
                40,
                31,
                "But those who wait for Yahweh will renew their strength. They will mount up with wings like eagles. They will run, and not be weary. They will walk, and not faint."
            ),
            v!(
                43,
                3,
                16,
                "For God so loved the world, that he gave his only born Son, that whoever believes in him should not perish, but have eternal life."
            ),
            v!(
                43,
                3,
                17,
                "For God didn't send his Son into the world to judge the world, but that the world should be saved through him."
            ),
            v!(
                45,
                8,
                28,
                "We know that all things work together for good for those who love God, for those who are called according to his purpose."
            ),
            v!(
                45,
                8,
                31,
                "What then shall we say about these things? If God is for us, who can be against us?"
            ),
            v!(
                45,
                8,
                38,
                "For I am persuaded that neither death, nor life, nor angels, nor principalities, nor things present, nor things to come, nor powers,"
            ),
            v!(
                45,
                8,
                39,
                "nor height, nor depth, nor any other created thing will be able to separate us from God's love which is in Christ Jesus our Lord."
            ),
            v!(
                46,
                13,
                4,
                "Love is patient and is kind. Love doesn't envy. Love doesn't brag, is not proud,"
            ),
            v!(
                46,
                13,
                5,
                "doesn't behave itself inappropriately, doesn't seek its own way, is not provoked, takes no account of evil;"
            ),
            v!(
                46,
                13,
                6,
                "doesn't rejoice in unrighteousness, but rejoices with the truth;"
            ),
            v!(
                46,
                13,
                7,
                "bears all things, believes all things, hopes all things, and endures all things."
            ),
            v!(46, 13, 8, "Love never fails."),
            v!(
                50,
                4,
                6,
                "In nothing be anxious, but in everything, by prayer and petition with thanksgiving, let your requests be made known to God."
            ),
            v!(
                50,
                4,
                7,
                "And the peace of God, which surpasses all understanding, will guard your hearts and your thoughts in Christ Jesus."
            ),
            v!(
                66,
                21,
                4,
                "He will wipe away every tear from their eyes. Death will be no more; neither will there be mourning, nor crying, nor pain, any more. The first things have passed away."
            ),
        ],
    }
}

fn starter_kjv() -> ParsedBible {
    ParsedBible {
        code: "KJV".to_owned(),
        name: "King James Version (Starter)".to_owned(),
        language: "en".to_owned(),
        copyright: "Public Domain".to_owned(),
        verses: vec![
            v!(
                1,
                1,
                1,
                "In the beginning God created the heaven and the earth."
            ),
            v!(
                1,
                1,
                2,
                "And the earth was without form, and void; and darkness was upon the face of the deep. And the Spirit of God moved upon the face of the waters."
            ),
            v!(
                1,
                1,
                3,
                "And God said, Let there be light: and there was light."
            ),
            v!(19, 23, 1, "The LORD is my shepherd; I shall not want."),
            v!(
                19,
                23,
                2,
                "He maketh me to lie down in green pastures: he leadeth me beside the still waters."
            ),
            v!(
                19,
                23,
                3,
                "He restoreth my soul: he leadeth me in the paths of righteousness for his name's sake."
            ),
            v!(
                19,
                23,
                4,
                "Yea, though I walk through the valley of the shadow of death, I will fear no evil: for thou art with me; thy rod and thy staff they comfort me."
            ),
            v!(
                19,
                23,
                5,
                "Thou preparest a table before me in the presence of mine enemies: thou anointest my head with oil; my cup runneth over."
            ),
            v!(
                19,
                23,
                6,
                "Surely goodness and mercy shall follow me all the days of my life: and I will dwell in the house of the LORD for ever."
            ),
            v!(
                20,
                3,
                5,
                "Trust in the LORD with all thine heart; and lean not unto thine own understanding."
            ),
            v!(
                20,
                3,
                6,
                "In all thy ways acknowledge him, and he shall direct thy paths."
            ),
            v!(
                43,
                3,
                16,
                "For God so loved the world, that he gave his only begotten Son, that whosoever believeth in him should not perish, but have everlasting life."
            ),
            v!(
                43,
                3,
                17,
                "For God sent not his Son into the world to condemn the world; but that the world through him might be saved."
            ),
            v!(
                45,
                8,
                28,
                "And we know that all things work together for good to them that love God, to them who are the called according to his purpose."
            ),
            v!(
                46,
                13,
                4,
                "Charity suffereth long, and is kind; charity envieth not; charity vaunteth not itself, is not puffed up,"
            ),
            v!(
                46,
                13,
                5,
                "Doth not behave itself unseemly, seeketh not her own, is not easily provoked, thinketh no evil;"
            ),
            v!(
                46,
                13,
                6,
                "Rejoiceth not in iniquity, but rejoiceth in the truth;"
            ),
            v!(
                46,
                13,
                7,
                "Beareth all things, believeth all things, hopeth all things, endureth all things."
            ),
            v!(46, 13, 8, "Charity never faileth."),
            v!(
                50,
                4,
                6,
                "Be careful for nothing; but in every thing by prayer and supplication with thanksgiving let your requests be made known unto God."
            ),
            v!(
                50,
                4,
                7,
                "And the peace of God, which passeth all understanding, shall keep your hearts and minds through Christ Jesus."
            ),
        ],
    }
}
