# CEFR A1 core: short explanations for adjectives, adverbs, prepositions and quantifiers.
let s = import "../lib/schema.ncl" in
let ids = import "lib/ids.ncl" in
let expl = fun kc_id name lang_code text ex_a ex_b =>
  {
    id = "expl.a1.%{name}.short.%{lang_code}",
    target = { kind = "kc", id = kc_id },
    depth = "short",
    lang = lang_code,
    body = text,
    examples = [{ text = ex_a }, { text = ex_b }],
    review = { status = "draft" },
  }
in
{
  explanations = [
    expl ids.kc.adjective_position "adjective-position" "en"
      "A red car. The car is red. An adjective goes before a noun or after be; it does not take -s."
      "two small dogs" "The dogs are small.",
    expl ids.kc.adjective_position "adjective-position" "ru"
      "Красная машина. Машина красная. Прилагательное стоит перед существительным или после be и не получает -s."
      "две маленькие собаки" "Собаки маленькие.",
    expl ids.kc.adjective_position "adjective-position" "uz"
      "Qizil mashina. Mashina qizil. Sifat ot oldida yoki be dan keyin turadi va -s olmaydi."
      "ikkita kichkina it" "Itlar kichkina.",

    expl ids.kc.adverb_frequency "adverb-frequency" "en"
      "I always drink tea. She is never late. Frequency adverbs go before the main verb and after be."
      "We often walk here." "He usually works at home.",
    expl ids.kc.adverb_frequency "adverb-frequency" "ru"
      "Я всегда пью чай. Она никогда не опаздывает. Наречие частоты стоит перед смысловым глаголом и после be."
      "Мы часто здесь гуляем." "Он обычно работает дома.",
    expl ids.kc.adverb_frequency "adverb-frequency" "uz"
      "Men doim choy ichaman. U hech qachon kechikmaydi. Takrorlik ravishi asosiy feʼldan oldin va be dan keyin turadi."
      "Biz bu yerda tez-tez sayr qilamiz." "U odatda uyda ishlaydi.",

    expl ids.kc.preposition_place "preposition-place" "en"
      "The keys are in the bag. The cup is on the table. I am at the door. In = inside, on = surface, at = point."
      "She is in the kitchen." "The book is on the desk.",
    expl ids.kc.preposition_place "preposition-place" "ru"
      "Ключи в сумке. Чашка на столе. Я у двери. In — внутри, on — на поверхности, at — у точки."
      "Она на кухне." "Книга на столе.",
    expl ids.kc.preposition_place "preposition-place" "uz"
      "Kalitlar sumkada. Chashka stolda. Men eshik oldidaman. In — ichida, on — ustida, at — bir nuqtada."
      "U oshxonada." "Kitob stolda.",

    expl ids.kc.preposition_time "preposition-time" "en"
      "In July, on Monday, at seven o'clock. Use in for months and years, on for days and dates, at for clock times."
      "We meet on Friday." "The film starts at 8.",
    expl ids.kc.preposition_time "preposition-time" "ru"
      "В июле, в понедельник, в семь часов. In — месяцы и годы, on — дни и даты, at — время на часах."
      "Мы встречаемся в пятницу." "Фильм начинается в 8.",
    expl ids.kc.preposition_time "preposition-time" "uz"
      "Iyulda, dushanba kuni, soat yettida. In — oylar va yillar, on — kunlar va sanalar, at — soat vaqti uchun."
      "Biz juma kuni uchrashamiz." "Film soat 8 da boshlanadi.",

    expl ids.kc.some_any "some-any" "en"
      "I have some apples. I don't have any milk. Do you have any questions? Some = positive, any = negative and questions."
      "There is some bread." "She doesn't need any help.",
    expl ids.kc.some_any "some-any" "ru"
      "У меня есть яблоки. У меня нет молока. У тебя есть вопросы? Some — в утверждениях, any — в отрицаниях и вопросах."
      "Есть немного хлеба." "Ей не нужна помощь.",
    expl ids.kc.some_any "some-any" "uz"
      "Menda olma bor. Menda sut yoʻq. Savollaringiz bormi? Some — tasdiq gaplarda, any — inkor va savollarda."
      "Biroz non bor." "Unga yordam kerak emas.",

    expl ids.kc.much_many "much-many" "en"
      "How many books? How much water? Many goes with countable plurals, much with uncountable nouns."
      "I don't have much time." "There are many shops here.",
    expl ids.kc.much_many "much-many" "ru"
      "Сколько книг? Сколько воды? Many — с исчисляемыми во множественном числе, much — с неисчисляемыми."
      "У меня мало времени." "Здесь много магазинов.",
    expl ids.kc.much_many "much-many" "uz"
      "Qancha kitob? Qancha suv? Many — sanaladigan koʻplik bilan, much — sanalmaydigan otlar bilan."
      "Mening vaqtim kam." "Bu yerda doʻkonlar koʻp.",
  ],
} | s.File
