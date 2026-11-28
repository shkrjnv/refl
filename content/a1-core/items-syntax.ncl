# CEFR A1 core: starter items for continuous, modals, imperatives and questions.
let s = import "../lib/schema.ncl" in
let b = import "lib/build.ncl" in
let k = b.kc in
let m = b.mis in
{
  items = [
    b.choice_item {
      slug = "continuous-affirmative",
      kc = k.continuous_affirmative,
      difficulty = 1,
      stimulus = "I ___ now.",
      answer = "am reading",
      options = [
        { text = "read", misconception = null, confuses_with = k.present_affirmative },
        { text = "is reading", misconception = null },
      ],
      why = {
        en = "For an action happening now use am + -ing: I am reading.",
        ru = "Для действия сейчас — am + -ing: I am reading.",
        uz = "Hozir boʻlayotgan ish uchun am + -ing: I am reading.",
      },
    },
    b.gap_item {
      slug = "continuous-affirmative",
      kc = k.continuous_affirmative,
      difficulty = 2,
      instruction = "instr.gap-fill-form",
      stimulus = "She ___ (cook) dinner now.",
      answer = "is cooking",
      expected_wrong = [],
      why = {
        en = "With she use is + -ing: She is cooking.",
        ru = "С she — is + -ing: She is cooking.",
        uz = "She bilan is + -ing: She is cooking.",
      },
    },

    b.choice_item {
      slug = "continuous-questions",
      kc = k.continuous_questions,
      difficulty = 1,
      stimulus = "___ you listening?",
      answer = "Are",
      options = [
        { text = "Do", misconception = m.no_do },
        { text = "Is", misconception = null },
      ],
      why = {
        en = "Put are before you: Are you listening?",
        ru = "are ставится перед you: Are you listening?",
        uz = "are you dan oldin: Are you listening?",
      },
    },
    b.gap_item {
      slug = "continuous-questions",
      kc = k.continuous_questions,
      difficulty = 2,
      instruction = "instr.gap-fill-form",
      stimulus = "___ (be) he working now?",
      answer = "Is",
      expected_wrong = [
        { pattern = "Does", misconception = m.no_do },
        { pattern = "He is working now", misconception = m.word_order },
      ],
      why = {
        en = "Put is before he: Is he working now?",
        ru = "is ставится перед he: Is he working now?",
        uz = "is he dan oldin: Is he working now?",
      },
    },

    b.choice_item {
      slug = "can-ability",
      kc = k.can_ability,
      difficulty = 1,
      stimulus = "She ___ swim.",
      answer = "can",
      options = [
        { text = "cans", misconception = null },
        { text = "can to", misconception = null },
      ],
      why = {
        en = "can never takes -s or to: She can swim.",
        ru = "У can нет -s и to: She can swim.",
        uz = "can -s ham, to ham olmaydi: She can swim.",
      },
    },
    b.gap_item {
      slug = "can-ability",
      kc = k.can_ability,
      difficulty = 2,
      instruction = "instr.gap-fill-form",
      stimulus = "She can ___ (swim).",
      answer = "swim",
      expected_wrong = [],
      why = {
        en = "After can use the base verb: can swim.",
        ru = "После can — начальная форма: can swim.",
        uz = "can dan keyin asosiy shakl: can swim.",
      },
    },

    b.choice_item {
      slug = "can-requests",
      kc = k.can_requests,
      difficulty = 1,
      stimulus = "___ I open the window?",
      answer = "Can",
      options = [
        { text = "Do", misconception = m.no_do },
        { text = "Am", misconception = null },
      ],
      why = {
        en = "Use Can I + verb to ask permission: Can I open the window?",
        ru = "Can I + глагол — просьба о разрешении: Can I open the window?",
        uz = "Ruxsat soʻrash uchun Can I + feʼl: Can I open the window?",
      },
    },
    b.gap_item {
      slug = "can-requests",
      kc = k.can_requests,
      difficulty = 2,
      instruction = "instr.gap-fill-form",
      stimulus = "___ (can) you help me?",
      answer = "Can",
      expected_wrong = [
        { pattern = "Do", misconception = m.no_do },
      ],
      why = {
        en = "Can you + verb asks for help: Can you help me?",
        ru = "Can you + глагол — просьба о помощи: Can you help me?",
        uz = "Yordam soʻrash uchun Can you + feʼl: Can you help me?",
      },
    },

    b.choice_item {
      slug = "imperative",
      kc = k.imperative,
      difficulty = 1,
      stimulus = "___ the door.",
      answer = "Open",
      options = [
        { text = "Opens", misconception = null },
        { text = "To open", misconception = null },
      ],
      why = {
        en = "Use the base verb without a subject: Open the door.",
        ru = "Начальная форма без подлежащего: Open the door.",
        uz = "Ega qoʻshilmaydi, asosiy shakl: Open the door.",
      },
    },
    b.gap_item {
      slug = "imperative",
      kc = k.imperative,
      difficulty = 2,
      instruction = "instr.gap-fill-form",
      stimulus = "___ (not / touch) the wall.",
      answer = "Don't touch",
      accepted = ["Don't touch", "Do not touch"],
      expected_wrong = [],
      why = {
        en = "For a negative instruction use Don't + verb: Don't touch the wall.",
        ru = "Отрицательная инструкция — Don't + глагол: Don't touch the wall.",
        uz = "Inkor koʻrsatma uchun Don't + feʼl: Don't touch the wall.",
      },
    },

    b.choice_item {
      slug = "question-words",
      kc = k.question_words,
      difficulty = 1,
      stimulus = "___ do you live?",
      answer = "Where",
      options = [
        { text = "What", misconception = null },
        { text = "Who", misconception = null },
      ],
      why = {
        en = "Where asks about a place: Where do you live?",
        ru = "Where спрашивает о месте: Where do you live?",
        uz = "Where joy haqida soʻraydi: Where do you live?",
      },
    },
    b.gap_item {
      slug = "question-words",
      kc = k.question_words,
      difficulty = 2,
      instruction = "instr.gap-fill-form",
      stimulus = "___ (when) does the lesson start?",
      answer = "When",
      expected_wrong = [],
      why = {
        en = "When asks about time: When does the lesson start?",
        ru = "When спрашивает о времени: When does the lesson start?",
        uz = "When vaqt haqida soʻraydi: When does the lesson start?",
      },
    },
  ],
} | s.File
