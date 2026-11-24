let b = import "../lib/base.ncl" in
{
  subjects = b.subjects,
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations,
  # The lock holds `it.gap` version 1 with difficulty 1: the difficulty
  # changed together with the version, so the build passes and the bundle
  # keeps both projections.
  items = [
    b.gap { version = 2, difficulty = 2 },
    b.choice {
      options = [
        { text = "the", misconception = "en.m.two", confuses_with = "en.k.b" },
        { text = "—", misconception = "en.m.one" },
        { text = "an", misconception = null },
      ],
    },
  ],
}
