# Item instructions, translated once and referenced by id (docs/02, «Задание: общая часть»).
# No `review` record yet: every language is draft, and a draft is never shown.
# After a person has read a language, add
#   review = { ru = { status = "reviewed", by = "<name>" } }
# (docs/02, «Вычитка и показ»).
let s = import "lib/schema.ncl" in
{
  instructions = [
    {
      id = "instr.choose-option",
      text = {
        uz = "Toʻgʻri variantni tanlang.",
        ru = "Выберите правильный вариант.",
        en = "Choose the correct option.",
      },
    },
    {
      id = "instr.gap-fill-base",
      text = {
        uz = "Qavs ichidagi soʻzdan foydalanib, boʻsh joyni toʻldiring.",
        ru = "Заполните пропуск, используя слово в скобках.",
        en = "Fill in the gap using the word in brackets.",
      },
    },
    {
      id = "instr.gap-fill-article",
      text = {
        uz = "Qavsdagi soʻzni yozing, kerak boʻlsa a, an yoki the bilan.",
        ru = "Напишите слово в скобках, если нужно — с a, an или the.",
        en = "Write the word in brackets, with a, an or the if needed.",
      },
    },
    {
      id = "instr.gap-fill-form",
      text = {
        uz = "Qavsdagi soʻzni toʻgʻri shaklda yozing.",
        ru = "Напишите слово в скобках в правильной форме.",
        en = "Write the word in brackets in the correct form.",
      },
    },
    {
      id = "instr.find-error",
      text = {
        uz = "Gapdagi xatoni toping.",
        ru = "Найдите ошибку в предложении.",
        en = "Find the mistake in the sentence.",
      },
    },
  ],
} | s.File
