# Builders for the unit's items: every family states only its sentences,
# answers and diagnoses (docs/02, «Задание: общая часть», «Семьи заданий»).
# Files in lib/ are imported, not loaded as content.

# Review of the `why` and `hint` of every task of the unit, in one place:
# empty, so every language is draft and no task goes into a session
# (docs/02, «Вычитка и показ»). After a person has read a language, name them:
#   { ru = { status = "reviewed", by = "<name>", hash = "<from hashes.txt>" }, en = { ... } }
# The hash is of all tasks' why and hint in that language (key `group:articles`
# in `hashes.txt`); editing any of them is the build error `reviewed-text-changed`.
let task_review = {} in
{
  kc = {
    countability = "en.gram.noun.countability",
    a_an = "en.gram.article.a-an-sound",
    first = "en.gram.article.first-mention",
    second = "en.gram.article.second-mention",
    zero = "en.gram.article.zero-general",
  },
  mis = {
    omission = "en.mis.article.omission-l1",
    the_for_new = "en.mis.article.the-for-new",
    a_for_known = "en.mis.article.a-for-known",
    a_always = "en.mis.article.a-always",
    an_by_letter = "en.mis.article.an-by-letter",
    a_uncountable = "en.mis.noun.a-with-uncountable",
    plural_uncountable = "en.mis.noun.plural-uncountable",
    singular_after_number = "en.mis.noun.singular-after-number",
    the_for_general = "en.mis.article.the-for-general",
    a_plural = "en.mis.article.a-with-plural",
  },

  primary = fun id => { kc = id, role = "primary" },
  secondary = fun id => { kc = id, role = "secondary" },

  # a or an by the first sound. Nouns starting with a vowel letter but a
  # consonant sound (university, uniform) are written out by their families.
  article_of = fun noun =>
    if std.array.elem (std.string.substring 0 1 noun) ["a", "e", "i", "o", "u"] then "an" else "a",

  # One live item with one slot. `r` holds id, version, family, instruction,
  # difficulty, expected_time, stimulus and slot.
  item = fun kind r =>
    {
      id = r.id,
      version = r.version,
      subject = "en",
      family = r.family,
      type = kind,
      instruction = r.instruction,
      difficulty = r.difficulty,
      expected_time = r.expected_time,
      status = "live",
      review = task_review,
      # One review of all tasks' why and hint per language: one hash.
      review_group = "articles",
      stimulus = { text = r.stimulus },
      slots = [r.slot],
    },
}
