# CEFR A1 core: starter items for adjectives, adverbs, prepositions and quantifiers.
let s = import "../lib/schema.ncl" in
let b = import "lib/build.ncl" in
let k = b.kc in
let m = b.mis in
{
  items = [
    b.choice_item {
      slug = "adjective-position",
      kc = k.adjective_position,
      difficulty = 1,
      stimulus = "A ___ car.",
      answer = "red",
      options = [
        { text = "reds", misconception = null },
        { text = "redly", misconception = null },
      ],
      why = {
        en = "An adjective before a noun has no ending: a red car.",
        ru = "Прилагательное перед существительным не получает окончания: a red car.",
        uz = "Ot oldidagi sifat qoʻshimcha olmaydi: a red car.",
      },
    },
    b.gap_item {
      slug = "adjective-position",
      kc = k.adjective_position,
      difficulty = 2,
      instruction = "instr.gap-fill-form",
      stimulus = "The dogs are ___ (small).",
      answer = "small",
      expected_wrong = [],
      why = {
        en = "After be the adjective also has no ending: The dogs are small.",
        ru = "После be прилагательное тоже без окончания: The dogs are small.",
        uz = "be dan keyin ham sifat qoʻshimcha olmaydi: The dogs are small.",
      },
    },

    b.choice_item {
      slug = "adverb-frequency",
      kc = k.adverb_frequency,
      difficulty = 1,
      stimulus = "I ___ drink tea.",
      answer = "always",
      options = [
        { text = "am always", misconception = null },
        { text = "always am", misconception = null },
      ],
      why = {
        en = "Before the main verb use always: I always drink tea.",
        ru = "Перед смысловым глаголом — always: I always drink tea.",
        uz = "Asosiy feʼldan oldin always: I always drink tea.",
      },
    },
    b.gap_item {
      slug = "adverb-frequency",
      kc = k.adverb_frequency,
      difficulty = 2,
      instruction = "instr.gap-fill-form",
      stimulus = "She is ___ (never) late.",
      answer = "never",
      expected_wrong = [],
      why = {
        en = "After be use never: She is never late.",
        ru = "После be — never: She is never late.",
        uz = "be dan keyin never: She is never late.",
      },
    },

    b.choice_item {
      slug = "preposition-place",
      kc = k.preposition_place,
      difficulty = 1,
      stimulus = "The keys are ___ the bag.",
      answer = "in",
      options = [
        { text = "on", misconception = null },
        { text = "at", misconception = null },
      ],
      why = {
        en = "Inside something use in: in the bag.",
        ru = "Внутри чего-то — in: in the bag.",
        uz = "Biror narsaning ichida — in: in the bag.",
      },
    },
    b.gap_item {
      slug = "preposition-place",
      kc = k.preposition_place,
      difficulty = 2,
      instruction = "instr.gap-fill-form",
      stimulus = "The cup is ___ (on) the table.",
      answer = "on",
      expected_wrong = [],
      why = {
        en = "On a surface use on: on the table.",
        ru = "На поверхности — on: on the table.",
        uz = "Yuzada — on: on the table.",
      },
    },

    b.choice_item {
      slug = "preposition-time",
      kc = k.preposition_time,
      difficulty = 1,
      stimulus = "We meet ___ Monday.",
      answer = "on",
      options = [
        { text = "in", misconception = m.time_preposition, confuses_with = k.preposition_place },
        { text = "at", misconception = m.time_preposition, confuses_with = k.preposition_place },
      ],
      why = {
        en = "With a day use on: on Monday.",
        ru = "С днём недели — on: on Monday.",
        uz = "Hafta kuni bilan on: on Monday.",
      },
    },
    b.gap_item {
      slug = "preposition-time",
      kc = k.preposition_time,
      difficulty = 2,
      instruction = "instr.gap-fill-form",
      stimulus = "The film starts ___ (at) 8.",
      answer = "at",
      expected_wrong = [
        { pattern = "in", misconception = m.time_preposition, confuses_with = k.preposition_place },
        { pattern = "on", misconception = m.time_preposition },
      ],
      why = {
        en = "With a clock time use at: at 8.",
        ru = "С временем на часах — at: at 8.",
        uz = "Soat vaqti bilan at: at 8.",
      },
    },

    b.choice_item {
      slug = "some-any",
      kc = k.some_any,
      difficulty = 1,
      stimulus = "I have ___ apples.",
      answer = "some",
      options = [
        { text = "any", misconception = null },
        { text = "much", misconception = null },
      ],
      why = {
        en = "In a positive sentence use some: I have some apples.",
        ru = "В утверждении — some: I have some apples.",
        uz = "Tasdiq gapda some: I have some apples.",
      },
    },
    b.gap_item {
      slug = "some-any",
      kc = k.some_any,
      difficulty = 2,
      instruction = "instr.gap-fill-form",
      stimulus = "Do you have ___ (any) questions?",
      answer = "any",
      expected_wrong = [],
      why = {
        en = "In a question use any: Do you have any questions?",
        ru = "В вопросе — any: Do you have any questions?",
        uz = "Savolda any: Do you have any questions?",
      },
    },

    b.choice_item {
      slug = "much-many",
      kc = k.much_many,
      difficulty = 1,
      stimulus = "How ___ books do you have?",
      answer = "many",
      options = [
        { text = "much", misconception = m.much_many },
        { text = "any", misconception = null },
      ],
      why = {
        en = "Books is countable plural, so use many: many books.",
        ru = "Books — исчисляемое множественное, поэтому many: many books.",
        uz = "Books sanaladigan koʻplik, shuning uchun many: many books.",
      },
    },
    b.gap_item {
      slug = "much-many",
      kc = k.much_many,
      difficulty = 2,
      instruction = "instr.gap-fill-form",
      stimulus = "I don't have ___ (much) time.",
      answer = "much",
      expected_wrong = [
        { pattern = "many", misconception = m.much_many },
      ],
      why = {
        en = "Time is uncountable, so use much: much time.",
        ru = "Time неисчисляемое, поэтому much: much time.",
        uz = "Time sanalmaydi, shuning uchun much: much time.",
      },
    },
  ],
} | s.File
