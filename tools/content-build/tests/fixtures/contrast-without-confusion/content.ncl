# k.a and k.b are live and contrast, but no live item is marked
# confuses_with across the edge.
let b = import "../lib/base.ncl" in
let a = b.live_kc "k.a" in
let bb = b.live_kc "k.b" in
{
  kcs = b.kcs,
  edges = b.edges @ [{ kind = "contrasts", from = "k.a", to = "k.b" }],
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations @ a.explanations @ bb.explanations,
  items = a.items @ bb.items,
}
