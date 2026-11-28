# CEFR A1 core: starter items for be, there is/are and the present simple.
let s = import "../lib/schema.ncl" in
let b = import "lib/build.ncl" in
let k = b.kc in
let m = b.mis in
{
  items = [
    b.choice_item {
      slug = "be-affirmative",
      kc = k.be_affirmative,
      difficulty = 1,
      stimulus = "I ___ with you.",
      answer = "agree",
      options = [
        { text = "am agree", misconception = m.am_agree },
        { text = "is agree", misconception = m.am_agree },
      ],
      why = {
        en = "Agree is a verb, so it does not take am: I agree, not I am agree.",
        ru = "Agree — глагол, поэтому am не нужен: I agree, а не I am agree.",
        uz = "Agree — feʼl, shuning uchun am kerak emas: I agree, I am agree emas.",
      },
    },
    b.gap_item {
      slug = "be-affirmative",
      kc = k.be_affirmative,
      difficulty = 2,
      instruction = "instr.gap-fill-form",
      stimulus = "They ___ (be) at home.",
      answer = "are",
      expected_wrong = [
        { pattern = "at home", misconception = m.drop_copula },
      ],
      why = {
        en = "With they use are: They are at home.",
        ru = "С they — are: They are at home.",
        uz = "They bilan are: They are at home.",
      },
    },

    b.choice_item {
      slug = "be-negative",
      kc = k.be_negative,
      difficulty = 1,
      stimulus = "She ___ at work today.",
      answer = "isn't",
      options = [
        { text = "aren't", misconception = null },
        { text = "don't", misconception = null },
      ],
      why = {
        en = "With she use isn't: She isn't at work.",
        ru = "С she — isn't: She isn't at work.",
        uz = "She bilan isn't: She isn't at work.",
      },
    },
    b.gap_item {
      slug = "be-negative",
      kc = k.be_negative,
      difficulty = 2,
      instruction = "instr.gap-fill-form",
      stimulus = "I ___ (not / be) tired.",
      answer = "am not",
      accepted = ["am not", "'m not"],
      expected_wrong = [],
      why = {
        en = "Put not after am: I am not tired.",
        ru = "not ставится после am: I am not tired.",
        uz = "not am dan keyin: I am not tired.",
      },
    },

    b.choice_item {
      slug = "be-questions",
      kc = k.be_questions,
      difficulty = 1,
      stimulus = "___ you ready?",
      answer = "Are",
      options = [
        { text = "Do", misconception = m.no_do },
        { text = "Is", misconception = null },
      ],
      why = {
        en = "With you use are before the subject: Are you ready?",
        ru = "С you — are перед подлежащим: Are you ready?",
        uz = "You bilan are egadan oldin: Are you ready?",
      },
    },
    b.gap_item {
      slug = "be-questions",
      kc = k.be_questions,
      difficulty = 2,
      instruction = "instr.gap-fill-form",
      stimulus = "___ (be) she at home?",
      answer = "Is",
      expected_wrong = [
        { pattern = "Does", misconception = m.no_do },
        { pattern = "She is at home", misconception = m.word_order },
      ],
      why = {
        en = "Put is before she: Is she at home?",
        ru = "is ставится перед she: Is she at home?",
        uz = "is she dan oldin: Is she at home?",
      },
    },

    b.choice_item {
      slug = "there-is-are",
      kc = k.there_is_are,
      difficulty = 1,
      stimulus = "There ___ two cats in the garden.",
      answer = "are",
      options = [
        { text = "is", misconception = null },
        { text = "be", misconception = null },
      ],
      why = {
        en = "Two cats is plural, so there are.",
        ru = "Two cats — множественное число, поэтому there are.",
        uz = "Two cats koʻplik, shuning uchun there are.",
      },
    },
    b.gap_item {
      slug = "there-is-are",
      kc = k.there_is_are,
      difficulty = 2,
      instruction = "instr.gap-fill-form",
      stimulus = "There ___ (be) a shop near my house.",
      answer = "is",
      expected_wrong = [
        { pattern = "a shop near my house", misconception = m.drop_copula },
      ],
      why = {
        en = "One shop is singular, so there is.",
        ru = "Один магазин — единственное число, поэтому there is.",
        uz = "Bitta doʻkon birlik, shuning uchun there is.",
      },
    },

    b.choice_item {
      slug = "present-affirmative",
      kc = k.present_affirmative,
      difficulty = 1,
      stimulus = "I ___ in a school.",
      answer = "work",
      options = [
        { text = "works", misconception = null },
        { text = "working", misconception = null },
      ],
      why = {
        en = "With I use the base form: I work.",
        ru = "С I — начальная форма: I work.",
        uz = "I bilan feʼlning asosiy shakli: I work.",
      },
    },
    b.gap_item {
      slug = "present-affirmative",
      kc = k.present_affirmative,
      difficulty = 2,
      instruction = "instr.gap-fill-form",
      stimulus = "They ___ (live) in Samarkand.",
      answer = "live",
      expected_wrong = [],
      why = {
        en = "With they use the base form: They live.",
        ru = "С they — начальная форма: They live.",
        uz = "They bilan asosiy shakl: They live.",
      },
    },

    b.choice_item {
      slug = "present-third-person",
      kc = k.present_third_person,
      difficulty = 1,
      stimulus = "He ___ in a bank.",
      answer = "works",
      options = [
        { text = "work", misconception = m.third_person_s },
        { text = "working", misconception = null },
      ],
      why = {
        en = "With he add -s: He works.",
        ru = "С he добавляем -s: He works.",
        uz = "He bilan -s qoʻshiladi: He works.",
      },
    },
    b.gap_item {
      slug = "present-third-person",
      kc = k.present_third_person,
      difficulty = 2,
      instruction = "instr.gap-fill-form",
      stimulus = "My sister ___ (watch) films.",
      answer = "watches",
      expected_wrong = [
        { pattern = "watch", misconception = m.third_person_s },
        { pattern = "watchs", misconception = m.third_person_s },
      ],
      why = {
        en = "After ch add -es: watches.",
        ru = "После ch добавляем -es: watches.",
        uz = "ch dan keyin -es: watches.",
      },
    },

    b.choice_item {
      slug = "present-negative",
      kc = k.present_negative,
      difficulty = 1,
      stimulus = "She ___ live here.",
      answer = "doesn't",
      options = [
        { text = "don't", misconception = m.no_do },
        { text = "isn't", misconception = null },
      ],
      why = {
        en = "With she use doesn't: She doesn't live here.",
        ru = "С she — doesn't: She doesn't live here.",
        uz = "She bilan doesn't: She doesn't live here.",
      },
    },
    b.gap_item {
      slug = "present-negative",
      kc = k.present_negative,
      difficulty = 2,
      instruction = "instr.gap-fill-form",
      stimulus = "They ___ (not / work) on Sunday.",
      answer = "don't work",
      accepted = ["don't work", "do not work"],
      expected_wrong = [
        { pattern = "doesn't work", misconception = m.no_do },
        { pattern = "not work", misconception = m.no_do },
        { pattern = "don't works", misconception = m.third_person_s },
      ],
      why = {
        en = "With they use don't + base verb: They don't work.",
        ru = "С they — don't + начальная форма: They don't work.",
        uz = "They bilan don't + asosiy shakl: They don't work.",
      },
    },

    b.choice_item {
      slug = "present-questions",
      kc = k.present_questions,
      difficulty = 1,
      stimulus = "___ she work here?",
      answer = "Does",
      options = [
        { text = "Do", misconception = m.no_do },
        { text = "Is", misconception = null },
      ],
      why = {
        en = "With she use does: Does she work here?",
        ru = "С she — does: Does she work here?",
        uz = "She bilan does: Does she work here?",
      },
    },
    b.gap_item {
      slug = "present-questions",
      kc = k.present_questions,
      difficulty = 2,
      instruction = "instr.gap-fill-form",
      stimulus = "___ (do) you like tea?",
      answer = "Do",
      expected_wrong = [
        { pattern = "Does", misconception = m.no_do },
      ],
      why = {
        en = "With you use do: Do you like tea?",
        ru = "С you — do: Do you like tea?",
        uz = "You bilan do: Do you like tea?",
      },
    },
  ],
} | s.File
