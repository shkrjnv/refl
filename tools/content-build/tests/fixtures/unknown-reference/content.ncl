let b = import "../lib/base.ncl" in
{
  kcs = b.kcs,
  edges = b.edges @ [{ kind = "supports", from = "k.a", to = "k.missing" }],
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations,
  items = [
    b.gap {
      wrong = [
        { pattern = "cat", misconception = "m.one", confuses_with = "k.missing-gap" },
        { pattern = "the cat", misconception = "m.two" },
      ],
    },
    b.choice {
      options = [
        { text = "the", misconception = "m.two", confuses_with = "k.missing-choice" },
        { text = "—", misconception = "m.one" },
        { text = "an", misconception = null },
      ],
    },
  ],
}
