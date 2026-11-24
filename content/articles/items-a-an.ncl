# Topic «Articles»: items on a or an by the first sound
# (docs/15, «Состав темы «Артикли»»).
let s = import "../lib/schema.ncl" in
let b = import "lib/build.ncl" in
let kcs_aan = [b.primary b.kc.a_an, b.secondary b.kc.first] in
let sound_hint = {
  uz = "Soʻzni ovoz chiqarib ayting. Uning birinchi tovushi qanday?",
  ru = "Произнесите слово вслух. Какой в нём первый звук?",
  en = "Say the word aloud. What is its first sound?",
} in

# Why the article is a or an, by the kind of first sound.
let why_of = fun sound word =>
  if sound == "vowel" then {
    uz = "*%{word}* unli tovush bilan boshlanadi, shuning uchun *an %{word}*.",
    ru = "*%{word}* начинается с гласного звука, поэтому *an %{word}*.",
    en = "*%{word}* starts with a vowel sound, so *an %{word}*.",
  } else if sound == "silent-h" then {
    uz = "*%{word}* soʻzida h oʻqilmaydi, u unli tovush bilan boshlanadi: *an %{word}*.",
    ru = "В *%{word}* буква h не читается, слово начинается с гласного звука: *an %{word}*.",
    en = "The *h* in *%{word}* is silent, so it starts with a vowel sound: *an %{word}*.",
  } else {
    uz = "*%{word}* /j/ tovushi bilan boshlanadi, bu undosh: *a %{word}*.",
    ru = "*%{word}* начинается со звука /j/, это согласный: *a %{word}*.",
    en = "*%{word}* starts with the sound /j/, a consonant: *a %{word}*.",
  }
in
let article_by = fun sound => if sound == "yu" then "a" else "an" in
let other = fun art => if art == "an" then "a" else "an" in
# The wrong form: a before a plain vowel is `a always`; anything else goes
# by the letter.
let wrong_mis = fun sound => if sound == "vowel" then b.mis.a_always else b.mis.an_by_letter in

# Choice of a, an, the or nothing.
let choice = fun fam level sound word frame =>
  let art = article_by sound in
  b.item "choice" {
    id = "art.aan.%{fam}.%{word}",
    version = 1,
    family = "en.art.aan.%{fam}",
    instruction = "instr.choose-option",
    difficulty = level,
    expected_time = 10,
    stimulus = frame,
    slot = {
      kcs = kcs_aan,
      answer = art,
      accepted = [art],
      options =
        [
          { text = other art, misconception = wrong_mis sound },
          { text = "—", misconception = b.mis.omission },
        ]
        @ (if sound == "vowel" then [{ text = "the", misconception = b.mis.the_for_new }] else []),
      hint = sound_hint,
      why = why_of sound word,
    },
  }
in

# Gap fill: the learner writes the article and the word in brackets.
let gap = fun fam level sound slug word frame =>
  let art = article_by sound in
  b.item "gap_fill" {
    id = "art.aan.%{fam}.%{slug}",
    version = 1,
    family = "en.art.aan.%{fam}",
    instruction = "instr.gap-fill-article",
    difficulty = level,
    expected_time = 18,
    stimulus = frame,
    slot = {
      kcs = kcs_aan,
      answer = "%{art} %{word}",
      accepted = ["%{art} %{word}"],
      expected_wrong =
        [
          { pattern = "%{other art} %{word}", misconception = wrong_mis sound },
          { pattern = word, misconception = b.mis.omission },
        ]
        @ (if sound == "vowel" then [{ pattern = "the %{word}", misconception = b.mis.the_for_new }] else []),
      hint = sound_hint,
      why = why_of sound (std.string.split " " word |> std.array.first),
    },
  }
in

# Find the error: one a/an is wrong for the sound that follows.
let find = fun slug r =>
  b.item "find_error" {
    id = "art.aan.find.%{slug}",
    version = 1,
    family = "en.art.aan.find",
    instruction = "instr.find-error",
    difficulty = 3,
    expected_time = 20,
    stimulus = r.sentence,
    slot = {
      kcs = kcs_aan,
      answer = r.error,
      accepted = [r.error],
      options = r.distractors,
      hint = sound_hint,
      why = r.why,
    },
  }
in
{
  items = [
    choice "choice" 1 "vowel" "apple" "I'm eating ___ apple.",
    choice "choice" 1 "vowel" "egg" "She found ___ egg in the nest.",
    choice "choice" 1 "vowel" "orange" "I need ___ orange for the cake.",
    choice "sound" 3 "silent-h" "hour" "We waited for ___ hour.",
    choice "sound" 3 "silent-h" "honest" "He is ___ honest man.",
    choice "sound" 3 "yu" "university" "She studies at ___ university.",
    choice "sound" 3 "yu" "uniform" "Students at this school wear ___ uniform.",
    gap "gap" 2 "vowel" "elephant" "elephant" "Look! There is ___ (elephant) in the river.",
    gap "gap" 2 "vowel" "island" "island" "We saw ___ (island) from the plane.",
    gap "gap" 2 "vowel" "insect" "insect" "There is ___ (insect) on your arm.",
    gap "sound-gap" 4 "silent-h" "hour" "hour" "The film lasts ___ (hour).",
    gap "sound-gap" 4 "yu" "university" "university" "My brother goes to ___ (university) in Tashkent.",
    gap "sound-gap" 4 "silent-h" "honest-girl" "honest girl" "She is ___ (honest girl).",
    find "owl" {
      sentence = "At the zoo we saw a owl, an elephant and a horse.",
      error = "a owl",
      distractors = [
        { text = "an elephant", misconception = b.mis.a_always },
        { text = "a horse", misconception = b.mis.an_by_letter },
      ],
      why = why_of "vowel" "owl",
    },
    find "university" {
      sentence = "My uncle is an engineer and works at an university in a small town.",
      error = "an university",
      distractors = [
        { text = "an engineer", misconception = b.mis.a_always },
        { text = "a small town", misconception = b.mis.the_for_new },
      ],
      why = why_of "yu" "university",
    },
    find "hour" {
      sentence = "We waited a hour for an old bus at a small station.",
      error = "a hour",
      distractors = [
        { text = "an old bus", misconception = b.mis.a_always },
        { text = "a small station", misconception = b.mis.the_for_new },
      ],
      why = why_of "silent-h" "hour",
    },
  ],
} | s.File
