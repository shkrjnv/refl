# Passes with no diagnostics, but k.a stays draft: it has ten live tasks and
# one of them is not reviewed, so nine count (docs/02, «Вычитка и показ»).
let b = import "../lib/base.ncl" in
let a = b.live_kc "en.k.a" in
let unreview = fun i =>
  if i.id == "it.en.k.a.0" then std.record.update "review" {} i else i
in
{
  subjects = b.subjects,
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations @ a.explanations,
  items = std.array.map unreview a.items,
}
