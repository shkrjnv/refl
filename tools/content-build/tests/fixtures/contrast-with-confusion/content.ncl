# Passes: the k.a–k.b contrast is crossed by an item on k.b whose error
# pattern is the k.a answer (the edge is symmetric, so this direction
# counts). k.c has no contrast: it is draft for its own reasons.
let b = import "../lib/base.ncl" in
let a = b.live_kc "en.k.a" in
let bb = b.live_kc "en.k.b" in
{
  subjects = b.subjects,
  kcs = b.kcs @ [
    { id = "en.k.c", subject = "en", domain = "grammar", title = { en = "C" }, level = "A1", importance = 0.5, rule = "c" },
  ],
  edges = b.edges @ [
    { kind = "contrasts", from = "en.k.a", to = "en.k.b" },
  ],
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations @ a.explanations @ bb.explanations,
  items = a.items @ bb.items @ [
    b.gap {
      id = "it.confused",
      slot_kcs = [{ kc = "en.k.b", role = "primary" }],
      wrong = [
        { pattern = "cat", misconception = "en.m.one", confuses_with = "en.k.a" },
        { pattern = "the cat", misconception = "en.m.two" },
      ],
    },
  ],
}
