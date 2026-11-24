# Valid base content for the negative fixtures. Each fixture directory has
# one content.ncl that reuses these parts and introduces exactly one defect.
let why_text = { ru = "почему", en = "why" } in
let primary_kc = fun primary => [{ kc = primary, role = "primary" }] in
{
  kcs = [
    {
      id = "k.a",
      domain = "grammar",
      title = { en = "A" },
      cefr = "A1",
      importance = 0.5,
      rule = "a",
    },
    {
      id = "k.b",
      domain = "grammar",
      title = { en = "B" },
      cefr = "A1",
      importance = 0.5,
      rule = "b",
    },
  ],
  edges = [{ kind = "requires", from = "k.b", to = "k.a" }],
  misconceptions = [
    {
      id = "m.one",
      title = { en = "one" },
      affects = ["k.a"],
      origin = "l1-transfer",
      l1 = { uz = true, ru = true },
      prior = { uz = 0.5, ru = 0.5 },
      remedy = "k.a",
    },
    {
      id = "m.two",
      title = { en = "two" },
      affects = ["k.a"],
      origin = "overgeneralization",
      l1 = { uz = false, ru = false },
      prior = { uz = 0.5, ru = 0.5 },
      remedy = "k.a",
    },
  ],
  instructions = [
    { id = "i.gap", text = { ru = "заполните", en = "fill in" } },
    { id = "i.choice", text = { ru = "выберите", en = "choose" } },
  ],
  explanations = [
    {
      id = "e.m.one.ru",
      target = { kind = "misconception", id = "m.one" },
      depth = "short",
      lang = "ru",
      body = "один",
      review = { status = "draft" },
    },
    {
      id = "e.m.two.ru",
      target = { kind = "misconception", id = "m.two" },
      depth = "short",
      lang = "ru",
      body = "два",
      review = { status = "draft" },
    },
    {
      id = "e.m.one",
      target = { kind = "misconception", id = "m.one" },
      depth = "short",
      lang = "en",
      body = "one",
      review = { status = "draft" },
    },
    {
      id = "e.m.two",
      target = { kind = "misconception", id = "m.two" },
      depth = "short",
      lang = "en",
      body = "two",
      review = { status = "draft" },
    },
  ],

  # A live gap-fill item; every argument has a valid default.
  gap = fun args =>
    let a =
      {
        id | default = "it.gap",
        family | default = "f.gap",
        difficulty | default = 1,
        type | default = "gap_fill",
        status | default = "live",
        slot_kcs | default = primary_kc "k.a",
        answer | default = "a cat",
        accepted | default = ["(a|one) cat"],
        wrong | default = [
          { pattern = "cat", misconception = "m.one" },
          { pattern = "the cat", misconception = "m.two" },
        ],
        audio | default = null,
      }
      & args
    in
    {
      id = a.id,
      version = 1,
      family = a.family,
      type = a.type,
      instruction = "i.gap",
      stimulus = if a.audio == null then { text = "I have ___ (cat)." } else { text = "Listen.", audio = a.audio },
      slots = [
        {
          kcs = a.slot_kcs,
          answer = a.answer,
          accepted = a.accepted,
          expected_wrong = a.wrong,
          why = why_text,
        }
      ],
      difficulty = a.difficulty,
      expected_time = 20,
      status = a.status,
    },

  # What makes `kc` live: short explanations in ru and en and ten live
  # gap-fill items (production) in three families with difficulties 1 and 2.
  live_kc = fun kc =>
    let explanation = fun l =>
      {
        id = "e.%{kc}.%{l}",
        target = { kind = "kc", id = kc },
        depth = "short",
        lang = l,
        body = "rule",
        review = { status = "draft" },
      }
    in
    {
      explanations = [explanation "ru", explanation "en"],
      items =
        std.array.generate
          (fun i =>
            gap {
              id = "it.%{kc}.%{std.to_string i}",
              family = "f.%{kc}.%{std.to_string (i % 3)}",
              difficulty = 1 + i % 2,
              slot_kcs = primary_kc kc,
            }
          )
          10,
    },

  # A live choice item.
  choice = fun args =>
    let a =
      {
        id | default = "it.choice",
        options | default = [
        { text = "the", misconception = "m.two" },
        { text = "—", misconception = "m.one" },
        { text = "an", misconception = null },
        ],
      }
      & args
    in
    {
      id = a.id,
      version = 1,
      family = "f.choice",
      type = "choice",
      instruction = "i.choice",
      stimulus.text = "I have ___ cat.",
      slots = [{ kcs = primary_kc "k.a", answer = "a", accepted = ["a"], options = a.options, why = why_text }],
      difficulty = 1,
      expected_time = 10,
      status = "live",
    },
}
