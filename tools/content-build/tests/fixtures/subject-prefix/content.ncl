# A KC, a misconception and an item family without the `en.` prefix of
# their subject.
let b = import "../lib/base.ncl" in
{
  subjects = b.subjects,
  kcs = b.kcs @ [
    { id = "k.z", subject = "en", domain = "grammar", title = { en = "Z" }, level = "A1", importance = 0.5, rule = "z" },
  ],
  edges = b.edges,
  misconceptions = b.misconceptions @ [
    {
      id = "m.z",
      title = { en = "z" },
      affects = ["en.k.a"],
      origin = "l1-transfer",
      l1 = { uz = true, ru = true },
      prior = { uz = 0.5, ru = 0.5 },
      remedy = "en.k.a",
    },
  ],
  instructions = b.instructions,
  explanations = b.explanations,
  items = [b.gap {}, b.choice {}, b.gap { id = "it.z", family = "f.z" }],
}
