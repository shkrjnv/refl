let b = import "../lib/base.ncl" in
{
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations,
  items = [b.gap { slot_kcs = [{ kc = "k.a", role = "secondary" }] }],
}
