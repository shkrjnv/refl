# Item instructions, translated once and referenced by id (docs/02, «Задание: общая часть»).
let s = import "lib/schema.ncl" in
{
  instructions = [
    {
      id = "instr.choose-option",
      text = {
        uz = "To'g'ri variantni tanlang.",
        ru = "Выберите правильный вариант.",
        en = "Choose the correct option.",
      },
    },
    {
      id = "instr.gap-fill-base",
      text = {
        uz = "Qavs ichidagi so'zdan foydalanib, bo'sh joyni to'ldiring.",
        ru = "Заполните пропуск, используя слово в скобках.",
        en = "Fill in the gap using the word in brackets.",
      },
    },
  ],
} | s.File
