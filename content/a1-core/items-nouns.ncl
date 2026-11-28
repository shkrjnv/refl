# CEFR A1 core: starter items for nouns, pronouns and determiners.
# One recognition and one production task per KC; all draft.
let s = import "../lib/schema.ncl" in
let b = import "lib/build.ncl" in
let k = b.kc in
let m = b.mis in
{
  items = [
    b.choice_item {
      slug = "pronoun-subject",
      kc = k.pronoun_subject,
      difficulty = 1,
      stimulus = "___ works in a bank.",
      answer = "He",
      options = [
        { text = "Him", misconception = null },
        { text = "His", misconception = null },
      ],
      why = {
        en = "He is the subject before the verb; him and his cannot stand there.",
        ru = "He — подлежащее перед глаголом; him и his здесь стоять не могут.",
        uz = "He — feʼldan oldingi ega; him va his bu yerda turolmaydi.",
      },
    },
    b.gap_item {
      slug = "pronoun-subject",
      kc = k.pronoun_subject,
      difficulty = 1,
      instruction = "instr.gap-fill-base",
      stimulus = "___ (she) is my sister.",
      answer = "She",
      expected_wrong = [
        { pattern = "is my sister", misconception = m.drop_subject },
      ],
      why = {
        en = "English needs the subject: She is my sister, not Is my sister.",
        ru = "В английском нужно подлежащее: She is my sister, а не Is my sister.",
        uz = "Inglizchada ega kerak: She is my sister, Is my sister emas.",
      },
    },

    b.choice_item {
      slug = "pronoun-object",
      kc = k.pronoun_object,
      difficulty = 1,
      stimulus = "I call ___ every evening.",
      answer = "her",
      options = [
        { text = "she", misconception = null },
        { text = "hers", misconception = null },
      ],
      why = {
        en = "After the verb use the object form: I call her.",
        ru = "После глагола — форма дополнения: I call her.",
        uz = "Feʼldan keyin toʻldiruvchi shakli: I call her.",
      },
    },
    b.gap_item {
      slug = "pronoun-object",
      kc = k.pronoun_object,
      difficulty = 2,
      instruction = "instr.gap-fill-base",
      stimulus = "This gift is for ___ (they).",
      answer = "them",
      expected_wrong = [],
      why = {
        en = "After a preposition use them: for them.",
        ru = "После предлога — them: for them.",
        uz = "Predlogdan keyin them: for them.",
      },
    },

    b.choice_item {
      slug = "possessive-adjectives",
      kc = k.possessive_adjectives,
      difficulty = 1,
      stimulus = "This is ___ book.",
      answer = "my",
      options = [
        { text = "me", misconception = null },
        { text = "mine", misconception = null },
      ],
      why = {
        en = "Before a noun use my: my book.",
        ru = "Перед существительным — my: my book.",
        uz = "Ot oldida my: my book.",
      },
    },
    b.gap_item {
      slug = "possessive-adjectives",
      kc = k.possessive_adjectives,
      difficulty = 2,
      instruction = "instr.gap-fill-base",
      stimulus = "___ (she) children are at school.",
      answer = "Her",
      expected_wrong = [],
      why = {
        en = "Before a noun use her: her children.",
        ru = "Перед существительным — her: her children.",
        uz = "Ot oldida her: her children.",
      },
    },

    b.choice_item {
      slug = "possessive-s",
      kc = k.possessive_s,
      difficulty = 1,
      stimulus = "This is ___ bag.",
      answer = "Amina's",
      options = [
        { text = "Amina", misconception = null },
        { text = "of Amina", misconception = null },
      ],
      why = {
        en = "Amina's bag: 's shows that the bag belongs to Amina.",
        ru = "Amina's bag: 's показывает, что сумка принадлежит Амине.",
        uz = "Amina's bag: 's sumka Aminaga tegishli ekanini koʻrsatadi.",
      },
    },
    b.gap_item {
      slug = "possessive-s",
      kc = k.possessive_s,
      difficulty = 2,
      instruction = "instr.gap-fill-base",
      stimulus = "This is my ___ (brother) phone.",
      answer = "brother's",
      expected_wrong = [],
      why = {
        en = "My brother's phone: 's goes on the person, not on the thing.",
        ru = "My brother's phone: 's ставится на человека, а не на предмет.",
        uz = "My brother's phone: 's narsaga emas, odamga qoʻshiladi.",
      },
    },

    b.choice_item {
      slug = "plural-regular",
      kc = k.plural_regular,
      difficulty = 1,
      stimulus = "One box, two ___.",
      answer = "boxes",
      options = [
        { text = "boxs", misconception = null },
        { text = "boxies", misconception = null },
      ],
      why = {
        en = "After x add -es: boxes.",
        ru = "После x добавляем -es: boxes.",
        uz = "x dan keyin -es: boxes.",
      },
    },
    b.gap_item {
      slug = "plural-regular",
      kc = k.plural_regular,
      difficulty = 2,
      instruction = "instr.gap-fill-form",
      stimulus = "One city, two ___ (city).",
      answer = "cities",
      expected_wrong = [],
      why = {
        en = "Consonant + y becomes -ies: cities.",
        ru = "Согласная + y переходит в -ies: cities.",
        uz = "Undosh + y -ies ga aylanadi: cities.",
      },
    },

    b.choice_item {
      slug = "plural-irregular",
      kc = k.plural_irregular,
      difficulty = 1,
      stimulus = "One child, two ___.",
      answer = "children",
      options = [
        { text = "childs", misconception = null },
        { text = "childes", misconception = null },
      ],
      why = {
        en = "Child has an irregular plural: children.",
        ru = "У child особая форма множественного числа: children.",
        uz = "Child ning koʻpligi alohida: children.",
      },
    },
    b.gap_item {
      slug = "plural-irregular",
      kc = k.plural_irregular,
      difficulty = 2,
      instruction = "instr.gap-fill-form",
      stimulus = "One foot, two ___ (foot).",
      answer = "feet",
      expected_wrong = [],
      why = {
        en = "Foot has an irregular plural: feet.",
        ru = "У foot особая форма: feet.",
        uz = "Foot ning koʻpligi alohida: feet.",
      },
    },

    b.choice_item {
      slug = "demonstratives",
      kc = k.demonstratives,
      difficulty = 1,
      stimulus = "___ books are old.",
      answer = "These",
      options = [
        { text = "This", misconception = null },
        { text = "That", misconception = null },
      ],
      why = {
        en = "Books is plural, so use these.",
        ru = "Books — множественное число, поэтому these.",
        uz = "Books koʻplik, shuning uchun these.",
      },
    },
    b.gap_item {
      slug = "demonstratives",
      kc = k.demonstratives,
      difficulty = 2,
      instruction = "instr.gap-fill-base",
      stimulus = "___ (this) apples are sweet.",
      answer = "These",
      expected_wrong = [],
      why = {
        en = "Apples is plural, so use these, not this.",
        ru = "Apples — множественное число, поэтому these, а не this.",
        uz = "Apples koʻplik, shuning uchun these, this emas.",
      },
    },
  ],
} | s.File
