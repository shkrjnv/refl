# The same flag on a typed-input item is part of its kind and passes.
let s = import "../../../../../content/lib/schema.ncl" in
let b = import "../lib/base.ncl" in
{
  subjects = b.subjects,
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations,
  items = [(b.gap {}) & { case_sensitive = true }, b.choice {}],
} | s.File
