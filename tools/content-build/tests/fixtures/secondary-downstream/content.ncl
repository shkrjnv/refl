# The base graph has k.b requires k.a; one slot teaches k.a with k.b as a
# secondary KC, downstream of its primary. Fails with `secondary-downstream`
# only.
let b = import "../lib/base.ncl" in
{
  subjects = b.subjects,
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations,
  items = [
    b.gap {
      slot_kcs = [
        { kc = "en.k.a", role = "primary" },
        { kc = "en.k.b", role = "secondary" },
      ],
    },
    b.choice {},
  ],
}
