# Topic «Articles»: items. Each family is one construction with different
# sentences (docs/02, «Семьи заданий»); a function builds its members.
let s = import "../lib/schema.ncl" in
let first = "gram.article.first-mention" in
let second = "gram.article.second-mention" in
let countability = "gram.noun.countability" in
let omission = "mis.article.omission-l1" in
let the_for_new = "mis.article.the-for-new" in
let slot_kcs = fun primary =>
  [
    { kc = primary, role = "primary" },
    { kc = countability, role = "secondary" },
  ]
in
let article_of = fun noun => if std.array.elem (std.string.substring 0 1 noun) ["a", "e", "i", "o", "u"] then "an" else "a" in

# Choice: pick the article for a thing mentioned for the first time.
let choice = fun noun =>
  let art = article_of noun in
  {
    id = "art.fm.choice.%{noun}",
    version = 1,
    family = "art.fm.choice",
    type = "choice",
    instruction = "instr.choose-option",
    stimulus.text = "I have ___ %{noun}.",
    slots = [
      {
        kcs = slot_kcs first,
        answer = art,
        accepted = [art],
        options = [
          { text = "the", misconception = the_for_new },
          { text = "—", misconception = omission },
          { text = if art == "a" then "an" else "a", misconception = null },
        ],
        why = {
          ru = "%{noun} упоминается впервые, поэтому нужен a/an; %{art} — по первому звуку слова.",
          en = "%{noun} is mentioned for the first time, so it takes a/an; %{art} because of its first sound.",
        },
      }
    ],
    difficulty = 1,
    expected_time = 10,
    status = "live",
  }
in

# Gap fill with the base form: the learner writes the article and the noun.
let gap = fun family_id level frame noun =>
  let art = article_of noun in
  {
    id = "%{family_id}.%{noun}",
    version = 1,
    family = family_id,
    type = "gap_fill",
    instruction = "instr.gap-fill-base",
    stimulus.text = frame noun,
    slots = [
      {
        kcs = slot_kcs first,
        answer = "%{art} %{noun}",
        accepted = ["%{art} %{noun}"],
        expected_wrong = [
          { pattern = noun, misconception = omission },
          { pattern = "the %{noun}", misconception = the_for_new },
        ],
        why = {
          ru = "%{noun} здесь появляется впервые: %{art} %{noun}.",
          en = "This is the first mention of %{noun}: %{art} %{noun}.",
        },
      }
    ],
    difficulty = level,
    expected_time = 20,
    status = "live",
  }
in
let look = gap "art.fm.gap" 2 (fun noun => "Look! There is ___ (%{noun}) in the garden.") in
let bought = gap "art.fm.story" 3 (fun noun => "Yesterday I bought ___ (%{noun}). The %{noun} was very cheap.") in

# Second mention: still being written, so draft.
let second_gap = fun noun =>
  {
    id = "art.sm.gap.%{noun}",
    version = 1,
    family = "art.sm.gap",
    type = "gap_fill",
    instruction = "instr.gap-fill-base",
    stimulus.text = "I have a %{noun} and a bird. ___ (%{noun}) is black.",
    slots = [
      {
        kcs = slot_kcs second,
        answer = "The %{noun}",
        accepted = ["the %{noun}"],
        expected_wrong = [{ pattern = noun, misconception = omission }],
        why = {
          ru = "%{noun} уже упоминался, поэтому the.",
          en = "%{noun} has already been mentioned, so the.",
        },
      }
    ],
    difficulty = 2,
    expected_time = 20,
    status = "draft",
  }
in
{
  items =
    (std.array.map choice ["cat", "dog", "umbrella", "egg"])
    @ (std.array.map look ["bird", "owl", "ball", "apple"])
    @ (std.array.map bought ["umbrella", "bike", "hat"])
    @ (std.array.map second_gap ["dog", "cat"]),
} | s.File
