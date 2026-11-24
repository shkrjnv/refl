# Passes with no diagnostics. The test marks everything reviewed except
# (docs/02, «Вычитка и показ»): the short explanation of k.a in ru, the
# instruction i.gap in en and the micro-explanation of en.m.one in en. Then
# k.a stays draft: everything it needs exists, but not all of it is shown.
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
