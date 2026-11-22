let b = import "../lib/base.ncl" in
{
  kcs = b.kcs,
  edges = b.edges @ [{ kind = "supports", from = "k.a", to = "k.missing" }],
  misconceptions = b.misconceptions,
  explanations = b.explanations,
  items = [b.gap {}, b.choice {}],
}
