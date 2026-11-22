let b = import "../lib/base.ncl" in
{
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  explanations = b.explanations,
  items = [b.gap { answer = "the cat" }],
}
