let b = import "../lib/base.ncl" in
{
  kcs = b.kcs,
  edges = b.edges @ [{ kind = "requires", from = "k.a", to = "k.b" }],
  misconceptions = b.misconceptions,
  explanations = b.explanations,
  items = [b.gap {}, b.choice {}],
}
