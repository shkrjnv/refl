# Topic «Articles»: misconceptions (docs/02, «Заблуждение»).
let s = import "../lib/schema.ncl" in
{
  misconceptions = [
    {
      id = "mis.article.omission-l1",
      title = {
        uz = "Artikl tushib qolgan",
        ru = "Пропуск артикля",
        en = "Missing article",
      },
      affects = ["gram.article.first-mention", "gram.article.second-mention"],
      origin = "l1-transfer",
      l1 = { uz = true, ru = true },
      prior = { uz = 0.8, ru = 0.75 },
      remedy = "gram.noun.countability",
    },
    {
      id = "mis.article.the-for-new",
      title = {
        uz = "Yangi narsa uchun the",
        ru = "the для нового предмета",
        en = "the for something new",
      },
      affects = ["gram.article.first-mention"],
      origin = "overgeneralization",
      l1 = { uz = false, ru = false },
      prior = { uz = 0.4, ru = 0.45 },
      remedy = "gram.article.first-mention",
    },
  ],
} | s.File
