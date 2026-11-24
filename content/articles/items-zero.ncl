# Topic «Articles»: items on no article for things in general
# (docs/15, «Состав темы «Артикли»»).
let s = import "../lib/schema.ncl" in
let b = import "lib/build.ncl" in
let kcs_zero = [b.primary b.kc.zero, b.secondary b.kc.countability] in
let zero_hint = {
  uz = "Gap aniq narsalar haqidami yoki umuman narsalar haqidami?",
  ru = "Речь о конкретных предметах или о предметах вообще?",
  en = "Is it about these particular things, or things in general?",
} in
let why_of = fun noun => {
  uz = "Gap umuman *%{noun}* haqida, shuning uchun artikl qoʻyilmaydi: *%{noun}*.",
  ru = "Речь о *%{noun}* вообще, поэтому без артикля: *%{noun}*.",
  en = "This is about %{noun} in general, so no article: *%{noun}*.",
} in
# `a` with a plural or with an uncountable noun: two different misconceptions.
let a_mis = fun kind => if kind == "plural" then b.mis.a_plural else b.mis.a_uncountable in
let capital = fun word =>
  "%{std.string.uppercase (std.string.substring 0 1 word)}%{std.string.substring 1 (std.string.length word) word}"
in

# Choice among noun phrases; `start` puts the noun at the start of the sentence.
let choice = fun kind start noun frame =>
  let form = fun phrase => if start then capital phrase else phrase in
  b.item "choice" {
    id = "art.zero.choice.%{noun}",
    version = 1,
    family = "en.art.zero.choice",
    instruction = "instr.choose-option",
    difficulty = 2,
    expected_time = 12,
    stimulus = frame,
    slot = {
      kcs = kcs_zero,
      answer = form noun,
      accepted = [noun],
      options = [
        { text = form "the %{noun}", misconception = b.mis.the_for_general, confuses_with = b.kc.second },
        { text = form "a %{noun}", misconception = a_mis kind },
      ],
      hint = zero_hint,
      why = why_of noun,
    },
  }
in

# Gap fill: the learner writes the word in brackets, with an article only if
# it needs one.
let gap = fun fam level kind noun frame =>
  b.item "gap_fill" {
    id = "art.zero.%{fam}.%{noun}",
    version = 1,
    family = "en.art.zero.%{fam}",
    instruction = "instr.gap-fill-article",
    difficulty = level,
    expected_time = 18,
    stimulus = frame,
    slot = {
      kcs = kcs_zero,
      answer = noun,
      accepted = [noun],
      expected_wrong = [
        { pattern = "the %{noun}", misconception = b.mis.the_for_general, confuses_with = b.kc.second },
        { pattern = "a %{noun}", misconception = a_mis kind },
      ],
      hint = zero_hint,
      why = why_of noun,
    },
  }
in
let likes = gap "gap" 2 in
let general = gap "general" 3 in
{
  items = [
    choice "uncountable" true "water" "___ is important for life.",
    choice "plural" false "dogs" "I love ___. They are so friendly.",
    choice "uncountable" true "milk" "___ is good for your bones.",
    choice "plural" false "spiders" "My brother is afraid of ___.",
    likes "uncountable" "music" "I like ___ (music).",
    likes "plural" "cats" "___ (cats) are clever animals.",
    likes "uncountable" "sugar" "Too much ___ (sugar) is bad for your teeth.",
    likes "plural" "horses" "My little sister loves ___ (horses).",
    general "uncountable" "life" "___ (life) in a big city is expensive.",
    general "plural" "children" "___ (children) learn languages fast.",
    general "uncountable" "money" "___ (money) can't buy happiness.",
  ],
} | s.File
