# k.a names a domain its subject does not list.
let b = import "../lib/base.ncl" in
{
  subjects = b.subjects,
  kcs = std.array.map (fun k => if k.id == "k.a" then k & { domain | force = "phonology" } else k) b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations,
  items = [b.gap {}, b.choice {}],
}
