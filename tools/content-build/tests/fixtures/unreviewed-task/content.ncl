# Passes with no diagnostics. The test marks everything reviewed except the
# task it.en.k.a.0: k.a has ten live tasks and nine count, so it stays draft
# (docs/02, «Вычитка и показ»).
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
