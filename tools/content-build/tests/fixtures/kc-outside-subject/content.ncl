# An English item's slot needs a mathematics KC, and the mathematics
# subject names an English KC as its spelling KC.
let b = import "../lib/base.ncl" in
{
  subjects = b.subjects @ [
    {
      id = "math",
      title = { en = "Mathematics" },
      scale = { levels = [{ key = "g1", title = { en = "Grade 1" } }], step_logit = 0.8 },
      domains = ["arithmetic"],
      checker = "en-words",
      spelling_kc = "en.k.spelling",
    },
  ],
  kcs = b.kcs @ [
    { id = "math.k.x", subject = "math", domain = "arithmetic", title = { en = "X" }, level = "g1", importance = 0.5, rule = "x" },
  ],
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations,
  items = [
    b.gap { slot_kcs = [{ kc = "en.k.a", role = "primary" }, { kc = "math.k.x", role = "secondary" }] },
    b.choice {},
  ],
}
