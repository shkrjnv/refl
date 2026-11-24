# Subjects: scale of levels, KC domains and input checker (docs/10, «Предмет»).
let s = import "lib/schema.ncl" in
{
  subjects = [
    {
      id = "en",
      title = { uz = "Ingliz tili", ru = "Английский язык", en = "English" },
      scale = {
        levels = [
          { key = "pre-A1", title = { uz = "A1 gacha", ru = "До A1", en = "Pre-A1" } },
          { key = "A1", title = { uz = "A1", ru = "A1", en = "A1" } },
          { key = "A2", title = { uz = "A2", ru = "A2", en = "A2" } },
          { key = "B1", title = { uz = "B1", ru = "B1", en = "B1" } },
          { key = "B2", title = { uz = "B2", ru = "B2", en = "B2" } },
          { key = "C1", title = { uz = "C1", ru = "C1", en = "C1" } },
          { key = "C2", title = { uz = "C2", ru = "C2", en = "C2" } },
        ],
        step_logit = 1.2,
      },
      domains = [
        "grammar",
        "phonology",
        "orthography",
        "discourse",
        "reading-skill",
        "listening-skill",
        "writing-skill",
      ],
      checker = "en-words",
      spelling_kc = "en.orth.spelling",
    },
  ],
} | s.File
