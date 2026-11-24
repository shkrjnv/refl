# Passes with no diagnostics, but k.a and k.b stay draft: they contrast, and
# no live task is marked `confuses_with` across the edge (docs/02, «Проверки
# при сборке контента»). The edge blocks both ends, whatever else they have.
let b = import "../lib/base.ncl" in
let a = b.live_kc "en.k.a" in
let bb = b.live_kc "en.k.b" in
{
  subjects = b.subjects,
  kcs = b.kcs,
  edges = b.edges @ [{ kind = "contrasts", from = "en.k.a", to = "en.k.b" }],
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations @ a.explanations @ bb.explanations,
  items = a.items @ bb.items,
}
