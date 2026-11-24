# Valid base content for the negative fixtures. Each fixture directory has
# one content.ncl that reuses these parts and introduces exactly one defect.
let why_text = { ru = "почему", en = "why" } in
# Reviewed by a named person: draft texts keep a KC from going live (docs/02,
# «Вычитка и показ»), so the base content is reviewed.
let checked = { status = "reviewed", by = "test" } in
let both_checked = { ru = checked, en = checked } in
let primary_kc = fun primary => [{ kc = primary, role = "primary" }] in
{
  subjects = [
    {
      id = "en",
      title = { en = "English" },
      scale = {
        levels = [{ key = "A1", title = { en = "A1" } }, { key = "A2", title = { en = "A2" } }],
        step_logit = 1.2,
      },
      domains = ["grammar"],
      checker = "en-words",
      spelling_kc = "en.k.spelling",
    },
  ],
  kcs = [
    {
      id = "en.k.spelling",
      subject = "en",
      domain = "grammar",
      title = { en = "Spelling" },
      level = "A1",
      importance = 0.5,
      rule = "spelling",
    },
    {
      id = "en.k.a",
      subject = "en",
      domain = "grammar",
      title = { en = "A" },
      level = "A1",
      importance = 0.5,
      rule = "a",
    },
    {
      id = "en.k.b",
      subject = "en",
      domain = "grammar",
      title = { en = "B" },
      level = "A1",
      importance = 0.5,
      rule = "b",
    },
  ],
  edges = [{ kind = "requires", from = "en.k.b", to = "en.k.a" }],
  misconceptions = [
    {
      id = "en.m.one",
      title = { en = "one" },
      affects = ["en.k.a"],
      origin = "l1-transfer",
      l1 = { uz = true, ru = true },
      prior = { uz = 0.5, ru = 0.5 },
      remedy = "en.k.a",
    },
    {
      id = "en.m.two",
      title = { en = "two" },
      affects = ["en.k.a"],
      origin = "overgeneralization",
      l1 = { uz = false, ru = false },
      prior = { uz = 0.5, ru = 0.5 },
      remedy = "en.k.a",
    },
  ],
  instructions = [
    { id = "i.gap", text = { ru = "заполните", en = "fill in" }, review = both_checked },
    { id = "i.choice", text = { ru = "выберите", en = "choose" }, review = both_checked },
  ],
  explanations = [
    {
      id = "e.m.one.ru",
      target = { kind = "misconception", id = "en.m.one" },
      depth = "short",
      lang = "ru",
      body = "один",
      review = checked,
    },
    {
      id = "e.m.two.ru",
      target = { kind = "misconception", id = "en.m.two" },
      depth = "short",
      lang = "ru",
      body = "два",
      review = checked,
    },
    {
      id = "e.m.one",
      target = { kind = "misconception", id = "en.m.one" },
      depth = "short",
      lang = "en",
      body = "one",
      review = checked,
    },
    {
      id = "e.m.two",
      target = { kind = "misconception", id = "en.m.two" },
      depth = "short",
      lang = "en",
      body = "two",
      review = checked,
    },
  ],

  # A live gap-fill item; every argument has a valid default.
  gap = fun args =>
    let a =
      {
        id | default = "it.gap",
        version | default = 1,
        family | default = "en.f.gap",
        difficulty | default = 1,
        type | default = "gap_fill",
        status | default = "live",
        slot_kcs | default = primary_kc "en.k.a",
        answer | default = "a cat",
        accepted | default = ["(a|one) cat"],
        wrong | default = [
          { pattern = "cat", misconception = "en.m.one" },
          { pattern = "the cat", misconception = "en.m.two" },
        ],
        audio | default = null,
      }
      & args
    in
    {
      id = a.id,
      version = a.version,
      subject = "en",
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
        review = checked,
      }
    in
    {
      explanations = [explanation "ru", explanation "en"],
      items =
        std.array.generate
          (fun i =>
            gap {
              id = "it.%{kc}.%{std.to_string i}",
              family = "en.f.%{kc}.%{std.to_string (i % 3)}",
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
        { text = "the", misconception = "en.m.two" },
        { text = "—", misconception = "en.m.one" },
        { text = "an", misconception = null },
        ],
      }
      & args
    in
    {
      id = a.id,
      version = 1,
      subject = "en",
      family = "en.f.choice",
      type = "choice",
      instruction = "i.choice",
      stimulus.text = "I have ___ cat.",
      slots = [{ kcs = primary_kc "en.k.a", answer = "a", accepted = ["a"], options = a.options, why = why_text }],
      difficulty = 1,
      expected_time = 10,
      status = "live",
    },
}
