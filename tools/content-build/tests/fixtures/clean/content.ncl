let b = import "../lib/base.ncl" in
{
  subjects = b.subjects,
  kcs = b.kcs,
  edges = b.edges,
  misconceptions = std.array.map
    (fun m =>
      if m.id == "en.m.one" then
        m & {
          refutations = [
            {
              kind = "contrast",
              correct = "I have lost my keys.",
              incorrect = "I lost my keys.",
              difference = { ru = "ключей нет сейчас", en = "the keys are missing now" },
            },
            {
              kind = "counterexample",
              claim = "a before every vowel letter",
              counterexample = "a university",
              explanation = { ru = "звук [ju]", en = "it starts with the sound [ju]" },
            },
          ],
        }
      else
        m
    )
    b.misconceptions,
  instructions = b.instructions,
  explanations = std.array.map
    (fun e =>
      if e.id == "e.m.one" then
        e & {
          examples = [
            { text = "I have a cat.", representation = "pictorial" },
          ],
        }
      else
        e
    )
    b.explanations,
  items = [
    b.gap {} & { representation = "concrete" },
    b.choice {
      options = [
        { text = "the", misconception = "en.m.two", confuses_with = "en.k.b" },
        { text = "—", misconception = "en.m.one" },
        { text = "an", misconception = null },
      ],
    } & { representation = "abstract" },
  ],
}
