# CEFR A1 core: nouns, pronouns and determiners.
# Every KC is a single testable rule (docs/02, «Единица знания и граф»).
let s = import "../lib/schema.ncl" in
let ids = import "lib/ids.ncl" in
{
  kcs = [
    {
      id = ids.kc.pronoun_subject,
      subject = "en",
      domain = "grammar",
      title = {
        uz = "Ega olmoshlari",
        ru = "Личные местоимения — подлежащее",
        en = "Subject pronouns",
      },
      level = "A1",
      importance = 0.95,
      rule = "I, you, he, she, it, we, they stand before the verb; English needs a subject in every sentence.",
      tags = ["a1-core", "l1-hard-uz", "l1-hard-ru"],
    },
    {
      id = ids.kc.pronoun_object,
      subject = "en",
      domain = "grammar",
      title = {
        uz = "Toʻldiruvchi olmoshlari",
        ru = "Личные местоимения — дополнение",
        en = "Object pronouns",
      },
      level = "A1",
      importance = 0.75,
      rule = "me, you, him, her, it, us, them come after a verb or a preposition.",
      tags = ["a1-core"],
    },
    {
      id = ids.kc.possessive_adjectives,
      subject = "en",
      domain = "grammar",
      title = {
        uz = "Egalik olmoshlari",
        ru = "Притяжательные прилагательные",
        en = "my, your, his, her, its, our, their",
      },
      level = "A1",
      importance = 0.85,
      rule = "my, your, his, her, its, our, their come before a noun and do not change with the noun's number.",
      tags = ["a1-core"],
    },
    {
      id = ids.kc.possessive_s,
      subject = "en",
      domain = "grammar",
      title = {
        uz = "Egalik maʼnosidagi 's",
        ru = "Притяжательный 's",
        en = "Possessive 's",
      },
      level = "A1",
      importance = 0.70,
      rule = "Add 's to a person or animal to show possession; use of for things.",
      tags = ["a1-core"],
    },
    {
      id = ids.kc.plural_regular,
      subject = "en",
      domain = "grammar",
      title = {
        uz = "Muntazam koʻplik",
        ru = "Правильное множественное число",
        en = "Regular plurals",
      },
      level = "A1",
      importance = 0.90,
      rule = "Add -s; add -es after s, x, ch, sh; a consonant + y becomes -ies.",
      tags = ["a1-core", "l1-hard-uz", "l1-hard-ru"],
    },
    {
      id = ids.kc.plural_irregular,
      subject = "en",
      domain = "grammar",
      title = {
        uz = "Notoʻgʻri koʻplik",
        ru = "Неправильное множественное число",
        en = "Irregular plurals",
      },
      level = "A1",
      importance = 0.65,
      rule = "man → men, woman → women, child → children, foot → feet, tooth → teeth, person → people.",
      tags = ["a1-core"],
    },
    {
      id = ids.kc.demonstratives,
      subject = "en",
      domain = "grammar",
      title = {
        uz = "Koʻrsatish olmoshlari",
        ru = "Указательные местоимения",
        en = "this, that, these, those",
      },
      level = "A1",
      importance = 0.75,
      rule = "this/that go with a singular noun; these/those go with a plural noun.",
      tags = ["a1-core", "l1-hard-uz"],
    },
  ],
} | s.File
