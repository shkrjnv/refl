# Defect: an explanation is `reviewed` but names no reviewer. Review is a
# person (docs/02, «Вычитка и показ»).
let b = import "../lib/base.ncl" in
{
  subjects = b.subjects,
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions @ [
    {
      id = "i.nameless",
      text = { ru = "текст", en = "text" },
      review = { ru = { status = "reviewed" } },
    },
  ],
  explanations = b.explanations @ [
    {
      id = "e.k.a.en",
      target = { kind = "kc", id = "en.k.a" },
      depth = "short",
      lang = "en",
      body = "rule",
      review = { status = "reviewed" },
    },
  ],
}
