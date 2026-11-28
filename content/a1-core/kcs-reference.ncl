# CEFR A1 core: adjectives, adverbs, prepositions and quantifiers.
let s = import "../lib/schema.ncl" in
let ids = import "lib/ids.ncl" in
{
  kcs = [
    {
      id = ids.kc.adjective_position,
      subject = "en",
      domain = "grammar",
      title = {
        uz = "Sifatning oʻrni",
        ru = "Место прилагательного",
        en = "Adjective position",
      },
      level = "A1",
      importance = 0.75,
      rule = "An adjective goes before a noun or after be; it does not take -s.",
      tags = ["a1-core"],
    },
    {
      id = ids.kc.adverb_frequency,
      subject = "en",
      domain = "grammar",
      title = {
        uz = "Takrorlik ravishlari",
        ru = "Наречия частоты",
        en = "Adverbs of frequency",
      },
      level = "A1",
      importance = 0.65,
      rule = "always, usually, often, sometimes, never go before the main verb and after be.",
      tags = ["a1-core"],
    },
    {
      id = ids.kc.preposition_place,
      subject = "en",
      domain = "grammar",
      title = {
        uz = "Joy predloglari: in, on, at",
        ru = "Предлоги места: in, on, at",
        en = "in, on, at: place",
      },
      level = "A1",
      importance = 0.85,
      rule = "in for enclosed spaces, on for surfaces, at for points: in the box, on the table, at the door.",
      tags = ["a1-core", "l1-hard-uz", "l1-hard-ru"],
    },
    {
      id = ids.kc.preposition_time,
      subject = "en",
      domain = "grammar",
      title = {
        uz = "Vaqt predloglari: in, on, at",
        ru = "Предлоги времени: in, on, at",
        en = "in, on, at: time",
      },
      level = "A1",
      importance = 0.80,
      rule = "in for months, years and parts of the day; on for days and dates; at for clock times.",
      tags = ["a1-core", "l1-hard-uz", "l1-hard-ru"],
    },
    {
      id = ids.kc.some_any,
      subject = "en",
      domain = "grammar",
      title = {
        uz = "some va any",
        ru = "some и any",
        en = "some and any",
      },
      level = "A1",
      importance = 0.75,
      rule = "Use some in positive sentences and offers; use any in negatives and questions.",
      tags = ["a1-core"],
    },
    {
      id = ids.kc.much_many,
      subject = "en",
      domain = "grammar",
      title = {
        uz = "much va many",
        ru = "much и many",
        en = "much and many",
      },
      level = "A1",
      importance = 0.70,
      rule = "Use many with countable plurals and much with uncountable nouns.",
      tags = ["a1-core", "l1-hard-uz", "l1-hard-ru"],
    },
  ],
} | s.File
