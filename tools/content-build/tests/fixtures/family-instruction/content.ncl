# A gap-fill item joins the family of the choice item, whose instruction is
# another one. Fails with `family-instruction` only.
let b = import "../lib/base.ncl" in
{
  subjects = b.subjects,
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations,
  items = [
    b.gap {},
    b.gap { id = "it.gap2", family = "en.f.choice" },
    b.choice {},
  ],
}
