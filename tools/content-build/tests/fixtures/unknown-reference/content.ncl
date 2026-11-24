let b = import "../lib/base.ncl" in
{
  subjects = b.subjects,
  kcs = b.kcs @ [
    { id = "xx.k.q", subject = "xx", domain = "grammar", title = { en = "Q" }, level = "A1", importance = 0.5, rule = "q" },
  ],
  edges = b.edges @ [{ kind = "supports", from = "en.k.a", to = "en.k.missing" }],
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations,
  items = [
    b.gap {
      wrong = [
        { pattern = "cat", misconception = "en.m.one", confuses_with = "en.k.missing-gap" },
        { pattern = "the cat", misconception = "en.m.two" },
      ],
    },
    b.choice {
      options = [
        { text = "the", misconception = "en.m.two", confuses_with = "en.k.missing-choice" },
        { text = "—", misconception = "en.m.one" },
        { text = "an", misconception = null },
      ],
    },
  ],
}
