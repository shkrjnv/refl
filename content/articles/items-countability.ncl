# Topic «Articles»: items on countable and uncountable nouns
# (docs/15, «Состав темы «Артикли»»).
let s = import "../lib/schema.ncl" in
let b = import "lib/build.ncl" in
let kcs_count = [b.primary b.kc.countability] in
let count_hint = {
  uz = "Buni sanash mumkinmi: bir, ikki, uch?",
  ru = "Это можно посчитать: один, два, три?",
  en = "Can you count it: one, two, three?",
} in
let uncountable_why = fun noun => {
  uz = "*%{noun}* sanalmaydi: *a/an* ham, *-s* ham qoʻshilmaydi, *some %{noun}* deyiladi.",
  ru = "*%{noun}* не считается: без *a/an* и без *-s*, говорят *some %{noun}*.",
  en = "You can't count *%{noun}*: no *a/an*, no *-s*. Say *some %{noun}*.",
} in

# Choice among noun phrases: an uncountable noun takes some, not a/an or -s.
let choice = fun noun frame =>
  b.item "choice" {
    id = "art.count.choice.%{noun}",
    version = 1,
    family = "en.art.count.choice",
    instruction = "instr.choose-option",
    difficulty = 1,
    expected_time = 12,
    stimulus = frame,
    slot = {
      kcs = kcs_count,
      answer = "some %{noun}",
      accepted = ["some %{noun}"],
      options = [
        { text = "%{b.article_of noun} %{noun}", misconception = b.mis.a_uncountable },
        { text = "%{noun}s", misconception = b.mis.plural_uncountable },
      ],
      hint = count_hint,
      why = uncountable_why noun,
    },
  }
in

# Gap fill after a number: the plural.
let number = fun noun count frame =>
  b.item "gap_fill" {
    id = "art.count.number.%{noun}",
    version = 1,
    family = "en.art.count.number",
    instruction = "instr.gap-fill-form",
    difficulty = 2,
    expected_time = 15,
    stimulus = frame,
    slot = {
      kcs = kcs_count,
      answer = "%{noun}s",
      accepted = ["[%{count}] %{noun}s"],
      expected_wrong = [
        { pattern = "[%{count}] %{noun}", misconception = b.mis.singular_after_number },
        { pattern = "a %{noun}s", misconception = b.mis.a_plural },
      ],
      hint = {
        uz = "Boʻsh joydan oldingi songa qarang.",
        ru = "Посмотрите на число перед пропуском.",
        en = "Look at the number before the gap.",
      },
      why = {
        uz = "*%{count}* soʻzidan keyin ot koʻplikda turadi: *%{count} %{noun}s*.",
        ru = "После *%{count}* существительное стоит во множественном числе: *%{count} %{noun}s*.",
        en = "After *%{count}* the noun is plural: *%{count} %{noun}s*.",
      },
    },
  }
in

# Gap fill with an uncountable noun: no a/an, no -s. `the` is not a
# countability error, so it is accepted too.
let uncountable = fun noun frame =>
  b.item "gap_fill" {
    id = "art.count.gap.%{noun}",
    version = 1,
    family = "en.art.count.gap",
    instruction = "instr.gap-fill-article",
    difficulty = 3,
    expected_time = 18,
    stimulus = frame,
    slot = {
      kcs = kcs_count,
      answer = "some %{noun}",
      accepted = ["[(some|the)] %{noun}"],
      expected_wrong = [
        { pattern = "(a|an) %{noun}", misconception = b.mis.a_uncountable },
        { pattern = "[(some|the)] %{noun}s", misconception = b.mis.plural_uncountable },
      ],
      hint = count_hint,
      why = uncountable_why noun,
    },
  }
in

# Find the error: one noun phrase breaks the countability rule; flagging a
# correct phrase shows the misconception that makes it look wrong.
let find = fun noun r =>
  b.item "find_error" {
    id = "art.count.find.%{noun}",
    version = 1,
    family = "en.art.count.find",
    instruction = "instr.find-error",
    difficulty = 3,
    expected_time = 20,
    stimulus = r.sentence,
    slot = {
      kcs = kcs_count,
      answer = r.error,
      accepted = [r.error],
      options = r.distractors,
      hint = {
        uz = "Har bir otni tekshiring: uni sanash mumkinmi?",
        ru = "Проверьте каждое существительное: его можно посчитать?",
        en = "Check each noun: can you count it?",
      },
      why = r.why,
    },
  }
in
{
  items = [
    choice "water" "Can I have ___, please? I'm thirsty.",
    choice "advice" "My teacher gave me ___ about the exam.",
    choice "furniture" "We bought ___ for the new flat.",
    choice "information" "I need ___ about the bus times.",
    number "chair" "three" "There are three ___ (chair) in the room.",
    number "apple" "two" "I ate two ___ (apple) after lunch.",
    number "book" "five" "My bag has five ___ (book) in it.",
    number "cup" "four" "Please put four ___ (cup) on the table.",
    uncountable "information" "Where can I get ___ (information) about the trip?",
    uncountable "advice" "Can you give me ___ (advice)?",
    uncountable "homework" "We have ___ (homework) for tomorrow.",
    uncountable "luggage" "She has ___ (luggage) with her.",
    find "furniture" {
      sentence = "We bought three chairs, a lamp and some new furnitures.",
      error = "some new furnitures",
      distractors = [
        { text = "three chairs", misconception = b.mis.singular_after_number },
        { text = "a lamp", misconception = b.mis.omission },
      ],
      why = {
        uz = "*Furniture* koʻplik shakliga ega emas: *some new furniture*.",
        ru = "У *furniture* нет множественного числа: *some new furniture*.",
        en = "*Furniture* has no plural: *some new furniture*.",
      },
    },
    find "advice" {
      sentence = "My sister gave me an advice, two books and a pen.",
      error = "an advice",
      distractors = [
        { text = "two books", misconception = b.mis.singular_after_number },
        { text = "a pen", misconception = b.mis.omission },
      ],
      why = {
        uz = "*Advice* sanalmaydi: *an advice* emas, *some advice*.",
        ru = "*Advice* не считается: *some advice*, а не *an advice*.",
        en = "*Advice* can't be counted: *some advice*, not *an advice*.",
      },
    },
    find "bread" {
      sentence = "For breakfast I have an egg, two apples and a bread.",
      error = "a bread",
      distractors = [
        { text = "an egg", misconception = b.mis.a_always },
        { text = "two apples", misconception = b.mis.singular_after_number },
      ],
      why = {
        uz = "*Bread* sanalmaydi: *a bread* emas, *some bread*.",
        ru = "*Bread* не считается: *some bread*, а не *a bread*.",
        en = "*Bread* can't be counted: *some bread*, not *a bread*.",
      },
    },
  ],
} | s.File
