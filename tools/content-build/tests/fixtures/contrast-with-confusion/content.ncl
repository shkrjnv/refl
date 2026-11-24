# Passes: the k.a–k.b contrast is crossed by an item on k.b whose error
# pattern is the k.a answer (the edge is symmetric, so this direction
# counts). The k.a–k.c contrast needs nothing: k.c is draft.
let b = import "../lib/base.ncl" in
let a = b.live_kc "k.a" in
let bb = b.live_kc "k.b" in
{
  subjects = b.subjects,
  kcs = b.kcs @ [
    { id = "k.c", subject = "en", domain = "grammar", title = { en = "C" }, level = "A1", importance = 0.5, rule = "c" },
  ],
  edges = b.edges @ [
    { kind = "contrasts", from = "k.a", to = "k.b" },
    { kind = "contrasts", from = "k.a", to = "k.c" },
  ],
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations @ a.explanations @ bb.explanations,
  items = a.items @ bb.items @ [
    b.gap {
      id = "it.confused",
      slot_kcs = [{ kc = "k.b", role = "primary" }],
      wrong = [
        { pattern = "cat", misconception = "m.one", confuses_with = "k.a" },
        { pattern = "the cat", misconception = "m.two" },
      ],
    },
  ],
}
