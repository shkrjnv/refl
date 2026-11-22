let b = import "../lib/base.ncl" in
{
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  explanations = b.explanations,
  items = [b.gap { slot_kcs = [{ kc = "k.a", role = "secondary" }] }],
}
