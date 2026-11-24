# Passes with no diagnostics, but k.a stays draft: its title is reviewed in
# ru and only drafted in en (docs/02, «Вычитка и показ»).
let b = import "../lib/base.ncl" in
let a = b.live_kc "en.k.a" in
let drafted = fun k =>
  if k.id == "en.k.a" then
    std.record.update "review" { ru = { status = "reviewed", by = "test" } } k
  else
    k
in
{
  subjects = b.subjects,
  kcs = std.array.map drafted b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations @ a.explanations,
  items = a.items,
}
