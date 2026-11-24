# A choice item carries an English checker flag: the per-kind item contract
# of the real schema rejects it (only typed-input kinds have these flags).
let s = import "../../../../../content/lib/schema.ncl" in
let b = import "../lib/base.ncl" in
{
  subjects = b.subjects,
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations,
  items = [b.gap {}, (b.choice {}) & { case_sensitive = true }],
} | s.File
