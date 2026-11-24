# Topic «Articles»: items. Each family is one construction with different
# sentences (docs/02, «Семьи заданий»); a function builds its members.
# This file: first mention (a/an). The other KCs of the unit have their own
# items-*.ncl files.
let s = import "../lib/schema.ncl" in
let b = import "lib/build.ncl" in
let vowel = fun noun => b.article_of noun == "an" in

# KCs of a first-mention slot: a vowel noun also needs the a/an-by-sound rule.
let kcs_of = fun noun =>
  [b.primary b.kc.first, b.secondary b.kc.countability]
  @ (if vowel noun then [b.secondary b.kc.a_an] else [])
in
let why_of = fun noun =>
  let art = b.article_of noun in
  let sound = if vowel noun then {
    uz = " *An*, chunki *%{noun}* unli tovush bilan boshlanadi.",
    ru = " *An* — потому что *%{noun}* начинается с гласного звука.",
    en = " *An*, because *%{noun}* starts with a vowel sound.",
  } else { uz = "", ru = "", en = "" }
  in
  {
    uz = "*%{noun}* bu yerda birinchi marta tilga olinyapti, shuning uchun *%{art} %{noun}*.%{sound.uz}",
    ru = "*%{noun}* здесь упоминается впервые, поэтому *%{art} %{noun}*.%{sound.ru}",
    en = "This is the first time we hear about the %{noun}, so it's *%{art} %{noun}*.%{sound.en}",
  }
in
let first_hint = {
  uz = "Bu narsa haqida birinchi marta gapirilyaptimi?",
  ru = "Об этом предмете говорят впервые?",
  en = "Is this the first time we hear about this thing?",
} in

# Choice: pick the article for a thing mentioned for the first time.
# Version 2: `the` is marked as the second-mention answer; the wrong a/an
# before a vowel sound is diagnosed; vowel nouns need a-an-sound.
let choice = fun ver noun =>
  let art = b.article_of noun in
  b.item "choice" {
    id = "art.fm.choice.%{noun}",
    version = ver,
    family = "en.art.fm.choice",
    instruction = "instr.choose-option",
    difficulty = 1,
    expected_time = 10,
    stimulus = "I have ___ %{noun}.",
    slot = {
      kcs = kcs_of noun,
      answer = art,
      accepted = [art],
      options = [
        { text = "the", misconception = b.mis.the_for_new, confuses_with = b.kc.second },
        { text = "—", misconception = b.mis.omission },
        if vowel noun then
          { text = "a", misconception = b.mis.a_always }
        else
          { text = "an", misconception = null },
      ],
      hint = first_hint,
      why = why_of noun,
    },
  }
in

# Gap fill with the base form: the learner writes the article and the noun.
# Version 2 of the first members: the article instruction, `the` marked as
# the second-mention answer, `a` before a vowel sound diagnosed.
let gap = fun fam level frame ver noun =>
  let art = b.article_of noun in
  b.item "gap_fill" {
    id = "%{fam}.%{noun}",
    version = ver,
    family = "en.%{fam}",
    instruction = "instr.gap-fill-article",
    difficulty = level,
    expected_time = 20,
    stimulus = frame noun,
    slot = {
      kcs = kcs_of noun,
      answer = "%{art} %{noun}",
      accepted = ["%{art} %{noun}"],
      expected_wrong =
        [
          { pattern = noun, misconception = b.mis.omission },
          { pattern = "the %{noun}", misconception = b.mis.the_for_new, confuses_with = b.kc.second },
        ]
        @ (if vowel noun then [{ pattern = "a %{noun}", misconception = b.mis.a_always }] else []),
      hint = first_hint,
      why = why_of noun,
    },
  }
in
let look = gap "art.fm.gap" 2 (fun noun => "Look! There is ___ (%{noun}) in the garden.") in
let bought = gap "art.fm.story" 3 (fun noun => "Yesterday I bought ___ (%{noun}). The %{noun} was very cheap.") in
{
  items =
    (std.array.map (choice 2) ["cat", "dog", "umbrella", "egg"])
    @ [choice 1 "bike"]
    @ (std.array.map (look 2) ["bird", "owl", "ball", "apple"])
    @ (std.array.map (bought 2) ["umbrella", "bike", "hat"]),
} | s.File
