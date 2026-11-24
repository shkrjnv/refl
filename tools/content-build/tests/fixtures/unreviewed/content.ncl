# Passes with no diagnostics, but k.a stays draft: everything it needs
# exists, none of it is shown yet (docs/02, «Вычитка и показ»).
#  - the short explanation in ru is draft;
#  - the instruction i.gap is reviewed in ru and draft in en;
#  - the micro-explanation of en.m.one in en is draft.
let b = import "../lib/base.ncl" in
let a = b.live_kc "en.k.a" in
let draft = fun e => std.record.update "review" { status = "draft" } e in
let is_draft = fun e => e.id == "e.en.k.a.ru" || e.id == "e.m.one" in
{
  subjects = b.subjects,
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = [
    {
      id = "i.gap",
      text = { ru = "заполните", en = "fill in" },
      review = {
        ru = { status = "reviewed", by = "test" },
        en = { status = "draft" },
      },
    },
    std.array.at 1 b.instructions,
  ],
  explanations =
    (std.array.map (fun e => if is_draft e then draft e else e) (b.explanations @ a.explanations)),
  items = a.items,
}
