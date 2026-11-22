let b = import "../lib/base.ncl" in
{
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = std.array.filter (fun e => e.id != "e.m.two") b.explanations,
  items = [b.gap {}],
}
