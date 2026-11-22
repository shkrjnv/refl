# Topic «Articles»: knowledge components and their graph (docs/02, «Единица знания и граф»).
let s = import "../lib/schema.ncl" in
{
  kcs = [
    {
      id = "gram.noun.countability",
      domain = "grammar",
      title = {
        uz = "Sanaladigan va sanalmaydigan otlar",
        ru = "Исчисляемые и неисчисляемые существительные",
        en = "Countable and uncountable nouns",
      },
      cefr = "A1",
      importance = 0.9,
      rule = "Countable nouns have a plural and take a/an in the singular; uncountable nouns take neither.",
      tags = ["l1-hard-uz", "l1-hard-ru"],
    },
    {
      id = "gram.article.first-mention",
      domain = "grammar",
      title = {
        uz = "a/an: birinchi eslatish",
        ru = "a/an: первое упоминание",
        en = "a/an: first mention",
      },
      cefr = "A1",
      importance = 0.95,
      rule = "A singular countable noun mentioned for the first time takes a (before a consonant sound) or an (before a vowel sound).",
      tags = ["l1-hard-uz", "l1-hard-ru"],
    },
    {
      id = "gram.article.second-mention",
      domain = "grammar",
      title = {
        uz = "the: takroriy eslatish",
        ru = "the: повторное упоминание",
        en = "the: second mention",
      },
      cefr = "A1",
      importance = 0.9,
      rule = "A noun that has already been mentioned, or is known to both speakers, takes the.",
      tags = ["l1-hard-uz", "l1-hard-ru"],
    },
  ],
  edges = [
    { kind = "requires", from = "gram.article.first-mention", to = "gram.noun.countability" },
    { kind = "requires", from = "gram.article.second-mention", to = "gram.article.first-mention" },
    { kind = "contrasts", from = "gram.article.first-mention", to = "gram.article.second-mention" },
  ],
} | s.File
