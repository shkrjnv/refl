let b = import "../lib/base.ncl" in
{
  subjects = b.subjects,
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations,
  items = [b.gap { wrong = [{ pattern = "[a] cat", misconception = "en.m.one" }, { pattern = "the cat", misconception = "en.m.two" }] }],
}
