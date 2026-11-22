let b = import "../lib/base.ncl" in
{
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations,
  items = [b.gap { wrong = [{ pattern = "cat", misconception = "m.one" }, { pattern = "[the] cat", misconception = "m.two" }] }],
}
