# The base graph has k.b requires k.a; a `supports` edge between them, in
# either direction, repeats it.
let b = import "../lib/base.ncl" in
{
  subjects = b.subjects,
  kcs = b.kcs,
  edges = b.edges @ [{ kind = "supports", from = "en.k.a", to = "en.k.b" }],
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations,
  items = [b.gap {}, b.choice {}],
}
