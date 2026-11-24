# Topic «Articles»: items on the for a thing already mentioned
# (docs/15, «Состав темы «Артикли»»).
let s = import "../lib/schema.ncl" in
let b = import "lib/build.ncl" in
let kcs_second = [b.primary b.kc.second, b.secondary b.kc.countability] in
let second_hint = {
  uz = "Bu narsa oldin tilga olinganmi?",
  ru = "Этот предмет уже упоминался?",
  en = "Was this thing mentioned before?",
} in
let why_of = fun noun => {
  uz = "*%{noun}* oldin tilga olingan, shuning uchun *the %{noun}*.",
  ru = "Про *%{noun}* уже говорили, поэтому *the %{noun}*.",
  en = "The %{noun} was already mentioned, so it's *the %{noun}*.",
} in

# Choice of The, A or nothing at the start of the second sentence.
let choice = fun noun frame =>
  b.item "choice" {
    id = "art.sm.choice.%{noun}",
    version = 1,
    family = "en.art.sm.choice",
    instruction = "instr.choose-option",
    difficulty = 1,
    expected_time = 12,
    stimulus = frame,
    slot = {
      kcs = kcs_second,
      answer = "The",
      accepted = ["the"],
      options = [
        { text = "A", misconception = b.mis.a_for_known, confuses_with = b.kc.first },
        { text = "—", misconception = b.mis.omission },
      ],
      hint = second_hint,
      why = why_of noun,
    },
  }
in

# Gap fill: the learner writes the article and the noun.
let gap = fun fam level ver noun frame =>
  b.item "gap_fill" {
    id = "art.sm.%{fam}.%{noun}",
    version = ver,
    family = "en.art.sm.%{fam}",
    instruction = "instr.gap-fill-article",
    difficulty = level,
    expected_time = 20,
    stimulus = frame,
    slot = {
      kcs = kcs_second,
      answer = "The %{noun}",
      accepted = ["the %{noun}"],
      expected_wrong = [
        { pattern = noun, misconception = b.mis.omission },
        { pattern = "a %{noun}", misconception = b.mis.a_for_known, confuses_with = b.kc.first },
      ],
      hint = second_hint,
      why = why_of noun,
    },
  }
in
# The first members were drafts at version 1; version 2 changes the
# instruction and adds the `a` pattern.
let pet = fun ver noun => gap "gap" 2 ver noun "I have a %{noun} and a bird. ___ (%{noun}) is black." in
let story = gap "story" 3 1 in

# Find the error: a noun mentioned before is introduced again with a.
let find = fun noun r =>
  b.item "find_error" {
    id = "art.sm.find.%{noun}",
    version = 1,
    family = "en.art.sm.find",
    instruction = "instr.find-error",
    difficulty = 3,
    expected_time = 20,
    stimulus = r.sentence,
    slot = {
      kcs = kcs_second,
      answer = r.error,
      accepted = [r.error],
      options = [
        { text = r.first, misconception = b.mis.the_for_new },
        { text = r.known, misconception = b.mis.a_for_known, confuses_with = b.kc.first },
      ],
      hint = second_hint,
      why = why_of noun,
    },
  }
in
{
  items = [
    choice "book" "I bought a book and a pen. ___ book is about space.",
    choice "shirt" "Dad gave me a shirt and a hat. ___ shirt is blue.",
    choice "cake" "We ordered a cake and a pizza. ___ cake was delicious.",
    choice "bag" "She has a bag and an umbrella. ___ bag is red.",
    pet 2 "dog",
    pet 2 "cat",
    pet 1 "rabbit",
    pet 1 "fish",
    story "garden" "My grandma has a garden and a dog. ___ (garden) is full of flowers.",
    story "sandwich" "I ordered a sandwich and a salad. ___ (sandwich) was too dry.",
    story "woman" "A man and a woman came into the shop. ___ (woman) asked for bread.",
    find "dog" {
      sentence = "We have a cat and a dog. The cat is old. A dog is young.",
      error = "A dog is young",
      first = "a cat and a dog",
      known = "The cat is old",
    },
    find "shirt" {
      sentence = "I bought a shirt and a hat. A shirt is blue, and the hat is red.",
      error = "A shirt is blue",
      first = "a shirt and a hat",
      known = "the hat is red",
    },
    find "boy" {
      sentence = "A boy and a girl came in. The girl sat down, and a boy stood by the window.",
      error = "a boy stood",
      first = "A boy and a girl",
      known = "The girl sat down",
    },
  ],
} | s.File
