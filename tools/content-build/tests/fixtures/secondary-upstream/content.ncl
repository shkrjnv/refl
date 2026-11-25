# What `secondary-downstream` allows: a slot of k.b with its prerequisite k.a
# as secondary (the base graph has k.b requires k.a), and a slot of k.a with
# k.c, joined to it by no edge. Builds without diagnostics.
let b = import "../lib/base.ncl" in
{
  subjects = b.subjects,
  kcs =
    b.kcs
    @ [
      {
        id = "en.k.c",
        subject = "en",
        domain = "grammar",
        title = { ru = "В", en = "C" },
        level = "A1",
        importance = 0.5,
        rule = "c",
      }
    ],
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations,
  items = [
    b.gap {
      id = "it.upstream",
      slot_kcs = [
        { kc = "en.k.b", role = "primary" },
        { kc = "en.k.a", role = "secondary" },
      ],
    },
    b.gap {
      id = "it.unrelated",
      slot_kcs = [
        { kc = "en.k.a", role = "primary" },
        { kc = "en.k.c", role = "secondary" },
      ],
    },
    b.choice {},
  ],
}
