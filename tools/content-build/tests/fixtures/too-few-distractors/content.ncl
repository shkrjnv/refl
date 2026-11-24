let b = import "../lib/base.ncl" in
{
  subjects = b.subjects,
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations,
  items = [b.choice { options = [{ text = "the", misconception = "m.two" }, { text = "an", misconception = null }] }],
}
