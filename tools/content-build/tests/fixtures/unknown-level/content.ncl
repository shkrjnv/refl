# k.a names a level its subject's scale does not have.
let b = import "../lib/base.ncl" in
{
  subjects = b.subjects,
  kcs = std.array.map (fun k => if k.id == "k.a" then k & { level | force = "B1" } else k) b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations,
  items = [b.gap {}, b.choice {}],
}
