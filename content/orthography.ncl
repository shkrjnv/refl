# Spelling: the KC a typo updates instead of the grammar rule
# (docs/02, «Проверка ответов на ввод»; docs/10, «Предмет»).
let s = import "lib/schema.ncl" in
{
  kcs = [
    {
      id = "en.orth.spelling",
      subject = "en",
      domain = "orthography",
      title = { uz = "Imlo", ru = "Орфография", en = "Spelling" },
      level = "A1",
      importance = 0.5,
      rule = "Words are spelled as in the dictionary. A typo of one or two letters counts here, not against the grammar rule of the task.",
      # Typos update it; it has no explanations or tasks of its own.
      observed_only = true,
    },
  ],
} | s.File
