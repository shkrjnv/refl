# CEFR A1 core: L1-driven misconceptions of Russian and Uzbek learners.
# Every entry is draft (no reviewed title yet) and is diagnosed by at least
# one starter item.
let s = import "../lib/schema.ncl" in
let ids = import "lib/ids.ncl" in
let k = ids.kc in
{
  misconceptions = [
    {
      id = ids.mis.drop_copula,
      title = {
        uz = "To be tushib qolgan",
        ru = "Пропущен глагол be",
        en = "be is missing",
      },
      affects = [k.be_affirmative, k.there_is_are],
      origin = "l1-transfer",
      l1 = { uz = true, ru = true },
      prior = { uz = 0.70, ru = 0.65 },
      remedy = k.be_affirmative,
    },
    {
      id = ids.mis.am_agree,
      title = {
        uz = "I am agree",
        ru = "I am agree",
        en = "I am agree",
      },
      affects = [k.be_affirmative],
      origin = "l1-transfer",
      l1 = { uz = true, ru = true },
      prior = { uz = 0.35, ru = 0.55 },
      remedy = k.be_affirmative,
    },
    {
      id = ids.mis.drop_subject,
      title = {
        uz = "Ega olmoshi tushib qolgan",
        ru = "Пропущено подлежащее-местоимение",
        en = "Subject pronoun is missing",
      },
      affects = [k.pronoun_subject, k.present_affirmative],
      origin = "l1-transfer",
      l1 = { uz = true, ru = true },
      prior = { uz = 0.60, ru = 0.50 },
      remedy = k.pronoun_subject,
    },
    {
      id = ids.mis.third_person_s,
      title = {
        uz = "he, she, it bilan -s yoʻq",
        ru = "Нет -s с he/she/it",
        en = "No -s with he, she, it",
      },
      affects = [k.present_third_person],
      origin = "l1-transfer",
      l1 = { uz = true, ru = true },
      prior = { uz = 0.75, ru = 0.50 },
      remedy = k.present_third_person,
    },
    {
      id = ids.mis.no_do,
      title = {
        uz = "do/does ishlatilmagan",
        ru = "Нет do/does",
        en = "No do/does",
      },
      affects = [k.present_negative, k.present_questions],
      origin = "l1-transfer",
      l1 = { uz = true, ru = true },
      prior = { uz = 0.70, ru = 0.60 },
      remedy = k.present_negative,
    },
    {
      id = ids.mis.word_order,
      title = {
        uz = "Savolda soʻz tartibi",
        ru = "Порядок слов в вопросе",
        en = "Question word order",
      },
      affects = [k.question_words, k.be_questions, k.present_questions],
      origin = "l1-transfer",
      l1 = { uz = true, ru = true },
      prior = { uz = 0.65, ru = 0.60 },
      remedy = k.question_words,
    },
    {
      id = ids.mis.time_preposition,
      title = {
        uz = "Vaqt predlogi xato",
        ru = "Неверный предлог времени",
        en = "Wrong time preposition",
      },
      affects = [k.preposition_time],
      origin = "l1-transfer",
      l1 = { uz = true, ru = true },
      prior = { uz = 0.50, ru = 0.65 },
      remedy = k.preposition_time,
    },
    {
      id = ids.mis.much_many,
      title = {
        uz = "much va many aralashgan",
        ru = "much и many перепутаны",
        en = "much and many mixed up",
      },
      affects = [k.much_many, "en.gram.noun.countability"],
      origin = "l1-transfer",
      l1 = { uz = true, ru = true },
      prior = { uz = 0.60, ru = 0.60 },
      remedy = k.much_many,
    },
  ],
} | s.File
