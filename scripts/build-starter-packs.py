#!/usr/bin/env python3
"""
Reproducible Public-Domain Starter Pack Builder for SundayDeck (.sdeck-pack)
serving churches across North America and South America.

Strictly copyright-compliant: compiles ONLY verified Public Domain / CC0 / open-licensed
Bibles, classic hymns, and traditional liturgical responses with explicit license &
attribution metadata. NEVER includes copyrighted contemporary worship songs or Bibles.

Usage:
    python3 scripts/build-starter-packs.py [--out-dir dist/starter-packs]
"""

from __future__ import annotations

import argparse
import json
import uuid
import zipfile
from pathlib import Path

SUNDAYDECK_NS = uuid.UUID("7b44e058-6e1c-4d75-b2e2-3a9483b74c19")


def det_id(tag: str) -> str:
    return str(uuid.uuid5(SUNDAYDECK_NS, tag))


def make_song(
    slug: str,
    title: str,
    author: str,
    primary_lang: str,
    secondary_lang: str | None,
    hymnal_entries: list[dict],
    traditions: list[str],
    stanzas: list[tuple[str, str, str, str | None]],
) -> dict:
    song_id = det_id(f"starter-pack:song:{slug}")
    sections = []
    sec_ids = []
    for idx, (kind, label, body, secondary_body) in enumerate(stanzas):
        sid = det_id(f"starter-pack:sec:{slug}:{idx}")
        sec_ids.append(sid)
        sec = {
            "id": sid,
            "kind": kind,
            "label": label,
            "ordinal": idx,
            "body": body,
        }
        if secondary_body:
            sec["secondary_body"] = secondary_body
        sections.append(sec)

    arr_id = det_id(f"starter-pack:arr:{slug}:standard")
    alt = " · ".join(f"{h['hymnal_code']} #{h['hymn_number']}" for h in hymnal_entries)
    song = {
        "id": song_id,
        "title": title,
        "alternate_title": alt,
        "author": author,
        "copyright": "Public Domain",
        "ccli_number": "",
        "default_theme_id": None,
        "default_arrangement_id": arr_id,
        "default_reflow_mode": "by_stanza",
        "primary_lang": primary_lang,
        "hymnal_entries": hymnal_entries,
        "traditions": traditions,
        "sections": sections,
        "arrangements": [
            {
                "id": arr_id,
                "name": "Standard",
                "section_ids": sec_ids,
            }
        ],
    }
    if secondary_lang:
        song["secondary_lang"] = secondary_lang
    return song


def write_pack(
    out_path: Path,
    pack_id: str,
    name: str,
    tradition: str,
    languages: list[str],
    attribution: str,
    bibles: list[dict],
    songs: list[dict],
    liturgies: list[dict],
) -> None:
    manifest = {
        "schema_version": 1,
        "pack_id": pack_id,
        "name": name,
        "version": "1.0.0",
        "tradition": tradition,
        "languages": languages,
        "license": "Public Domain",
        "attribution": attribution,
        "source_url": "https://ebible.org",
        "bible_count": len(bibles),
        "song_count": len(songs),
        "liturgy_count": len(liturgies),
    }
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(out_path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        zf.writestr("manifest.json", json.dumps(manifest, ensure_ascii=False, indent=2))
        zf.writestr("bibles.json", json.dumps(bibles, ensure_ascii=False, indent=2))
        zf.writestr("songs.json", json.dumps(songs, ensure_ascii=False, indent=2))
        zf.writestr("liturgies.json", json.dumps(liturgies, ensure_ascii=False, indent=2))


def build_all_packs(out_dir: Path) -> list[Path]:
    built: list[Path] = []

    # 1. North America Classic Hymns — Baptist, Methodist & Mainline (100% Public Domain)
    protestant_songs = [
        make_song(
            "o-for-a-thousand-tongues",
            "O for a Thousand Tongues to Sing",
            "Charles Wesley (1739)",
            "en",
            None,
            [
                {"hymnal_code": "UMH", "hymnal_name": "United Methodist Hymnal", "hymn_number": 57},
                {"hymnal_code": "BH", "hymnal_name": "Baptist Hymnal", "hymn_number": 216},
            ],
            ["Methodist", "Baptist", "Traditional Hymns"],
            [
                (
                    "verse",
                    "Verse 1",
                    "O for a thousand tongues to sing\nMy great Redeemer's praise,\nThe glories of my God and King,\nThe triumphs of his grace!",
                    None,
                ),
                (
                    "verse",
                    "Verse 2",
                    "Jesus! the name that charms our fears,\nThat bids our sorrows cease;\n'Tis music in the sinner's ears,\n'Tis life, and health, and peace.",
                    None,
                ),
            ],
        ),
        make_song(
            "holy-holy-holy",
            "Holy, Holy, Holy! Lord God Almighty",
            "Reginald Heber (1826)",
            "en",
            None,
            [
                {"hymnal_code": "UMH", "hymnal_name": "United Methodist Hymnal", "hymn_number": 64},
                {"hymnal_code": "BH", "hymnal_name": "Baptist Hymnal", "hymn_number": 1},
            ],
            ["Baptist", "Methodist", "Mainline"],
            [
                (
                    "verse",
                    "Verse 1",
                    "Holy, holy, holy! Lord God Almighty!\nEarly in the morning our song shall rise to Thee;\nHoly, holy, holy! Merciful and mighty!\nGod in three Persons, blessed Trinity!",
                    None,
                ),
            ],
        ),
    ]
    p1 = out_dir / "north-america-classic-hymns.sdeck-pack"
    write_pack(
        p1,
        "sundaydeck-north-america-hymns-pd",
        "Classic Public Domain Hymns (Baptist, Methodist & Mainline)",
        "Baptist & Methodist",
        ["en"],
        "Verified Public Domain hymns (Charles Wesley, Reginald Heber).",
        [],
        protestant_songs,
        [
            {
                "id": "methodist-apostles-creed",
                "title": "The Apostles' Creed (Responsive)",
                "tradition": "Methodist / Mainline",
                "language": "en",
                "lines": [
                    {
                        "speaker": "leader",
                        "text": "Where the Spirit of the Lord is, there is the one true church, apostolic and universal.",
                    },
                    {
                        "speaker": "all",
                        "text": "I believe in God, the Father Almighty, maker of heaven and earth; and in Jesus Christ his only Son our Lord.",
                    },
                ],
            }
        ],
    )
    built.append(p1)

    # 2. Catholic Parish 73-Book Canon + Order of Mass Responses (North & South America)
    p2 = out_dir / "catholic-parish-73book.sdeck-pack"
    write_pack(
        p2,
        "sundaydeck-catholic-73book-pd",
        "Catholic Parish Starter (73-Book Deuterocanon & Mass Responses)",
        "Catholic Parish",
        ["en", "es", "la"],
        "Catholic Public Domain Version (CPDV) & Traditional Order of Mass Responses.",
        [
            {
                "code": "CPDV",
                "name": "Catholic Public Domain Version (Starter)",
                "language": "en",
                "copyright": "Public Domain",
                "verses": [
                    [67, 4, 15, "See that you never do to another what you would hate to have done to yourself by another."],
                    [69, 3, 1, "But the souls of the just are in the hand of God, and the torment of death shall not touch them."],
                    [70, 2, 1, "Son, when you come to the service of God, stand in justice and in fear, and prepare your soul for temptation."],
                ],
            }
        ],
        [
            make_song(
                "holy-god-we-praise-thy-name",
                "Holy God, We Praise Thy Name",
                "Clarence A. Walworth (1853)",
                "en",
                None,
                [{"hymnal_code": "CATH", "hymnal_name": "Traditional Catholic Hymns", "hymn_number": 1}],
                ["Catholic"],
                [
                    (
                        "verse",
                        "Verse 1",
                        "Holy God, we praise Thy Name;\nLord of all, we bow before Thee!\nAll on earth Thy scepter claim,\nAll in Heaven above adore Thee!",
                        None,
                    )
                ],
            )
        ],
        [
            {
                "id": "catholic-mass-greeting",
                "title": "Order of Mass — Greeting & Kyrie (EN / ES)",
                "tradition": "Catholic",
                "language": "en",
                "lines": [
                    {
                        "speaker": "leader",
                        "text": "The Lord be with you.",
                        "secondary_text": "El Señor esté con ustedes.",
                    },
                    {
                        "speaker": "congregation",
                        "text": "And with your spirit.",
                        "secondary_text": "Y con tu espíritu.",
                    },
                    {
                        "speaker": "cantor",
                        "text": "Lord, have mercy.",
                        "secondary_text": "Kyrie, eleison.",
                    },
                    {
                        "speaker": "response",
                        "text": "Lord, have mercy.",
                        "secondary_text": "Kyrie, eleison.",
                    },
                ],
            }
        ],
    )
    built.append(p2)

    # 3. Spanish / Bilingual Himnario & Reina-Valera 1909 (Latin America & US Hispanic)
    p3 = out_dir / "spanish-himnario-rv1909.sdeck-pack"
    write_pack(
        p3,
        "sundaydeck-spanish-rv1909-pd",
        "Español / Bilingual Starter (Reina-Valera 1909 & Himnario Clásico)",
        "Spanish / Bilingual",
        ["es", "en"],
        "Reina-Valera 1909 (Public Domain) & classic Public Domain Spanish hymns.",
        [
            {
                "code": "RV1909",
                "name": "Reina-Valera 1909 (Starter)",
                "language": "es",
                "copyright": "Public Domain",
                "verses": [
                    [19, 23, 1, "Jehová es mi pastor; nada me faltará."],
                    [19, 23, 2, "En lugares de delicados pastos me hará yacer: junto á aguas de reposo me pastoreará."],
                    [43, 3, 16, "Porque de tal manera amó Dios al mundo, que ha dado á su Hijo unigénito, para que todo aquel que en él cree, no se pierda, mas tenga vida eterna."],
                ],
            }
        ],
        [
            make_song(
                "sublime-gracia",
                "Sublime Gracia (Amazing Grace)",
                "John Newton (1779) · Public Domain",
                "es",
                "en",
                [{"hymnal_code": "HGT", "hymnal_name": "Himnario Clásico", "hymn_number": 42}],
                ["Spanish / Alabanza", "Traditional Hymns"],
                [
                    (
                        "verse",
                        "Estrofa 1",
                        "Sublime gracia del Señor\nQue a un infeliz salvó;\nFui ciego mas hoy miro yo,\nPerdido y Él me halló.",
                        "Amazing grace! How sweet the sound\nThat saved a wretch like me!\nI once was lost, but now am found;\nWas blind, but now I see.",
                    )
                ],
            )
        ],
        [],
    )
    built.append(p3)

    # 4. Portuguese / Brazil Starter (Almeida Public Domain & Classic Hinário PT/EN)
    p4 = out_dir / "portuguese-almeida-hinos.sdeck-pack"
    write_pack(
        p4,
        "sundaydeck-portuguese-brazil-pd",
        "Português / Brasil Starter (Almeida Clássica & Hinos Tradicionais)",
        "Portuguese / Brazil",
        ["pt", "en"],
        "Public Domain João Ferreira de Almeida classic text & Public Domain hymns.",
        [
            {
                "code": "ALMPD",
                "name": "Almeida Clássica (Starter)",
                "language": "pt",
                "copyright": "Public Domain",
                "verses": [
                    [19, 23, 1, "O Senhor é o meu pastor; nada me faltará."],
                    [19, 23, 2, "Deitar-me faz em verdes pastos, guia-me mansamente a águas tranquilas."],
                    [43, 3, 16, "Porque Deus amou o mundo de tal maneira que deu o seu Filho unigênito, para que todo aquele que nele crê não pereça, mas tenha a vida eterna."],
                ],
            }
        ],
        [
            make_song(
                "santo-santo-santo-pt",
                "Santo! Santo! Santo! (Holy, Holy, Holy)",
                "Reginald Heber (1826) · Public Domain",
                "pt",
                "en",
                [
                    {"hymnal_code": "CC", "hymnal_name": "Cantor Cristão", "hymn_number": 1},
                    {"hymnal_code": "HC", "hymnal_name": "Hinário Clássico", "hymn_number": 15},
                ],
                ["Portuguese / Brazil", "Baptist", "Pentecostal"],
                [
                    (
                        "verse",
                        "Estrofe 1",
                        "Santo! Santo! Santo! Deus Onipotente!\nCedo de manhã cantaremos Teu louvor;\nSanto! Santo! Santo! Deus Jeová Triúno,\nÉs um só Deus, excelso Criador.",
                        "Holy, holy, holy! Lord God Almighty!\nEarly in the morning our song shall rise to Thee;\nHoly, holy, holy! Merciful and mighty!\nGod in three Persons, blessed Trinity!",
                    )
                ],
            )
        ],
        [],
    )
    built.append(p4)
    return built


def main() -> None:
    parser = argparse.ArgumentParser(description="Build Public Domain SundayDeck .sdeck-pack files.")
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path(__file__).resolve().parents[1] / "dist" / "starter-packs",
    )
    args = parser.parse_args()
    packs = build_all_packs(args.out_dir)
    for p in packs:
        print(f"Built {p} ({p.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
