# Defect: an explanation is reviewed with the hash of another text, as if
# its body was edited after the review (docs/02, «Вычитка и показ»).
let b = import "../lib/base.ncl" in
{
  subjects = b.subjects,
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations @ [
    {
      id = "e.k.a.edited",
      target = { kind = "kc", id = "en.k.a" },
      depth = "short",
      lang = "en",
      body = "edited after the review",
      review = {
        status = "reviewed",
        by = "test",
        hash = "0000000000000000000000000000000000000000000000000000000000000000",
      },
    },
  ],
}
