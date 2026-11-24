# en.k.a requires a KC of another subject. The `supports` edge across the
# same subjects is allowed and reports nothing.
let b = import "../lib/base.ncl" in
{
  subjects = b.subjects @ [
    {
      id = "math",
      title = { en = "Mathematics" },
      scale = { levels = [{ key = "g1", title = { en = "Grade 1" } }], step_logit = 0.8 },
      domains = ["arithmetic"],
      checker = "en-words",
    },
  ],
  kcs = b.kcs @ [
    { id = "math.k.x", subject = "math", domain = "arithmetic", title = { en = "X" }, level = "g1", importance = 0.5, rule = "x" },
  ],
  edges = b.edges @ [
    { kind = "requires", from = "en.k.a", to = "math.k.x" },
    { kind = "supports", from = "en.k.b", to = "math.k.x" },
  ],
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations,
  items = [b.gap {}, b.choice {}],
}
