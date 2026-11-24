# Topic «Articles»: explanations, one record per target, depth and language
# (docs/02, «Объяснение»).
let s = import "../lib/schema.ncl" in
let draft = { status = "draft" } in
{
  explanations = [
    {
      id = "expl.article.first-mention.short.ru",
      target = { kind = "kc", id = "en.gram.article.first-mention" },
      depth = "short",
      lang = "ru",
      body = "Когда исчисляемый предмет в единственном числе упоминается впервые, перед ним ставится **a** (перед согласным звуком) или **an** (перед гласным звуком).",
      l1_contrast = "В русском артиклей нет: «Я купил книгу» не говорит, новая это книга или уже известная. В английском это обязательно видно: I bought **a** book.",
      examples = [{ text = "I have a cat." }, { text = "She bought an umbrella." }],
      review = draft,
    },
    {
      id = "expl.article.first-mention.short.en",
      target = { kind = "kc", id = "en.gram.article.first-mention" },
      depth = "short",
      lang = "en",
      body = "When a singular countable thing is mentioned for the first time, put **a** before it (before a consonant sound) or **an** (before a vowel sound).",
      examples = [{ text = "I have a cat." }, { text = "She bought an umbrella." }],
      review = draft,
    },
    {
      id = "expl.article.second-mention.short.ru",
      target = { kind = "kc", id = "en.gram.article.second-mention" },
      depth = "short",
      lang = "ru",
      body = "Когда предмет уже упоминался, перед ним ставится **the**: I bought a book. **The** book is about space.",
      review = draft,
    },
    {
      id = "expl.article.second-mention.short.en",
      target = { kind = "kc", id = "en.gram.article.second-mention" },
      depth = "short",
      lang = "en",
      body = "Once a thing has been mentioned, use **the**: I bought a book. **The** book is about space.",
      review = draft,
    },
    {
      id = "expl.noun.countability.short.ru",
      target = { kind = "kc", id = "en.gram.noun.countability" },
      depth = "short",
      lang = "ru",
      body = "Исчисляемые существительные можно посчитать: a book, two books. Неисчисляемые — нельзя: water, advice; у них нет a/an и множественного числа.",
      review = draft,
    },
    {
      id = "expl.mis.article.omission-l1.ru",
      target = { kind = "misconception", id = "en.mis.article.omission-l1" },
      depth = "short",
      lang = "ru",
      body = "Перед исчисляемым существительным в единственном числе нужен артикль: не «I have cat», а «I have **a** cat».",
      review = draft,
    },
    {
      id = "expl.mis.article.omission-l1.en",
      target = { kind = "misconception", id = "en.mis.article.omission-l1" },
      depth = "short",
      lang = "en",
      body = "A singular countable noun needs an article: not \"I have cat\" but \"I have **a** cat\".",
      review = draft,
    },
    {
      id = "expl.mis.article.the-for-new.ru",
      target = { kind = "misconception", id = "en.mis.article.the-for-new" },
      depth = "short",
      lang = "ru",
      body = "**the** — для того, что уже известно. Новый предмет вводится через **a/an**.",
      review = draft,
    },
    {
      id = "expl.mis.article.the-for-new.en",
      target = { kind = "misconception", id = "en.mis.article.the-for-new" },
      depth = "short",
      lang = "en",
      body = "**the** is for things already known. A new thing is introduced with **a/an**.",
      review = draft,
    },
  ],
} | s.File
