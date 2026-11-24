# Passes with no diagnostics. The test marks everything reviewed except the
# title of k.a in en (docs/02, «Вычитка и показ»), so k.a stays draft.
let b = import "../lib/base.ncl" in
let a = b.live_kc "en.k.a" in
{
  subjects = b.subjects,
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations @ a.explanations,
  items = a.items,
}
