# CEFR A1 core: continuous, modals, imperatives and questions.
let s = import "../lib/schema.ncl" in
let ids = import "lib/ids.ncl" in
{
  kcs = [
    {
      id = ids.kc.continuous_affirmative,
      subject = "en",
      domain = "grammar",
      title = {
        uz = "Hozirgi davomiy zamon",
        ru = "Present Continuous",
        en = "am/is/are + -ing",
      },
      level = "A1",
      importance = 0.80,
      rule = "Use am/is/are + verb-ing for an action happening now: I am reading.",
      tags = ["a1-core"],
    },
    {
      id = ids.kc.continuous_questions,
      subject = "en",
      domain = "grammar",
      title = {
        uz = "Hozirgi davomiy zamon: savol",
        ru = "Present Continuous: вопросы",
        en = "Present continuous: questions",
      },
      level = "A1",
      importance = 0.65,
      rule = "Put am/is/are before the subject: Are you listening? Is he working?",
      tags = ["a1-core"],
    },
    {
      id = ids.kc.can_ability,
      subject = "en",
      domain = "grammar",
      title = {
        uz = "can: qobiliyat",
        ru = "can: умение",
        en = "can: ability",
      },
      level = "A1",
      importance = 0.85,
      rule = "can + base form for every subject; no -s and no do: She can swim.",
      tags = ["a1-core"],
    },
    {
      id = ids.kc.can_requests,
      subject = "en",
      domain = "grammar",
      title = {
        uz = "can: iltimos",
        ru = "can: просьба",
        en = "can: requests",
      },
      level = "A1",
      importance = 0.60,
      rule = "Can I/you + base form to ask for something or permission: Can I open the window?",
      tags = ["a1-core"],
    },
    {
      id = ids.kc.imperative,
      subject = "en",
      domain = "grammar",
      title = {
        uz = "Buyruq mayli",
        ru = "Повелительное наклонение",
        en = "Imperatives",
      },
      level = "A1",
      importance = 0.70,
      rule = "Use the base form without a subject for instructions and requests; use don't + base form for negatives.",
      tags = ["a1-core"],
    },
    {
      id = ids.kc.question_words,
      subject = "en",
      domain = "grammar",
      title = {
        uz = "Savol soʻzlari",
        ru = "Вопросительные слова",
        en = "Question words",
      },
      level = "A1",
      importance = 0.85,
      rule = "what, where, who, when, why, how ask for different information and come first.",
      tags = ["a1-core", "l1-hard-uz", "l1-hard-ru"],
    },
  ],
} | s.File
