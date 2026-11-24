let b = import "../lib/base.ncl" in
{
  subjects = b.subjects,
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = [
    { id = "i.gap", text = { ru = "заполните" } },
    { id = "i.choice", text = { ru = "выберите", en = "choose" } },
  ],
  explanations = b.explanations,
  items = [b.gap {}],
}
