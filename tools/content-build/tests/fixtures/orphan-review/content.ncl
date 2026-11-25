# The clean content with a reviews.lock line for a KC that does not exist.
let b = import "../lib/base.ncl" in
{
  subjects = b.subjects,
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations,
  items = [
    b.gap {},
    b.choice {
      options = [
        { text = "the", misconception = "en.m.two", confuses_with = "en.k.b" },
        { text = "—", misconception = "en.m.one" },
        { text = "an", misconception = null },
      ],
    },
  ],
}
