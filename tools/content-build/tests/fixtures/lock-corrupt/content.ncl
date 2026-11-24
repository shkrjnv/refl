let b = import "../lib/base.ncl" in
{
  subjects = b.subjects,
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations,
  # The lock lost the entry of `it.choice`; its digest no longer matches.
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
