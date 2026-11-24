# The subject's scale has a step of zero logits.
let b = import "../lib/base.ncl" in
{
  subjects = std.array.map (fun s => s & { scale.step_logit | force = 0 }) b.subjects,
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations,
  items = [b.gap {}, b.choice {}],
}
