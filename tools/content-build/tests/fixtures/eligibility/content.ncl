# Passes with no diagnostics. Session eligibility (docs/02, «Единица знания
# и граф»): a live item on live k.a is eligible; a draft item is not; a live
# item whose primary KC k.c is draft is not.
let b = import "../lib/base.ncl" in
let a = b.live_kc "en.k.a" in
{
  subjects = b.subjects,
  kcs = b.kcs @ [
    { id = "en.k.c", subject = "en", domain = "grammar", title = { en = "C" }, level = "A1", importance = 0.5, rule = "c" },
  ],
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations @ a.explanations,
  items = a.items @ [
    b.gap { id = "it.draft", status = "draft" },
    b.gap { id = "it.on-draft-kc", slot_kcs = [{ kc = "en.k.c", role = "primary" }] },
  ],
}
