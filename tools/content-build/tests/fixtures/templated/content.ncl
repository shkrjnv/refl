# Passes with no diagnostics. The review queue finds the source line of a
# text built from a template, and a review group has one hash per language
# (docs/02, «Вычитка и показ»). Nothing is reviewed.
let b = import "../lib/base.ncl" in
let why_of = fun noun =>
  {
    ru = "*%{noun}* здесь упоминается впервые.",
    en = "This is the first time we hear about the %{noun}.",
  }
in
let item = fun noun =>
  {
    id = "it.t.%{noun}",
    version = 1,
    subject = "en",
    family = "en.f.t",
    type = "gap_fill",
    instruction = "i.gap",
    review_group = "grp",
    stimulus = { text = "I have ___ (%{noun})." },
    slots = [
      {
        kcs = [{ kc = "en.k.a", role = "primary" }],
        answer = "a %{noun}",
        accepted = ["(a|one) %{noun}"],
        expected_wrong = [
          { pattern = "%{noun}", misconception = "en.m.one" },
          { pattern = "the %{noun}", misconception = "en.m.two" },
        ],
        why = why_of noun,
      }
    ],
    difficulty = 1,
    expected_time = 20,
    status = "live",
  }
in
{
  subjects = b.subjects,
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations,
  items = [item "cat", item "dog"],
}
