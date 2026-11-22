let b = import "../lib/base.ncl" in
{
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  explanations = b.explanations,
  items = [b.gap { wrong = [{ pattern = "cat", misconception = "m.one" }, { pattern = "[the] cat", misconception = "m.two" }] }],
}
