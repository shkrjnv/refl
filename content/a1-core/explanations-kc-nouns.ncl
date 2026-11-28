# CEFR A1 core: short explanations for nouns, pronouns and determiners.
# Draft until a person reviews the text (docs/02, «Вычитка и показ»).
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
    expl ids.kc.pronoun_subject "pronoun-subject" "en"
      "I am a student. They live in Tashkent. English needs the subject before the verb, even when the listener already knows who it is."
      "She works at a school." "We have two children.",
    expl ids.kc.pronoun_subject "pronoun-subject" "ru"
      "Я студент. Они живут в Ташкенте. В английском подлежащее стоит перед глаголом, даже если собеседник уже знает, о ком речь."
      "Она работает в школе." "У нас двое детей.",
    expl ids.kc.pronoun_subject "pronoun-subject" "uz"
      "Men talabaman. Ular Toshkentda yashaydi. Ingliz tilida ega har doim feʼldan oldin turadi, hatto kim haqida gapirilayotgani maʼlum boʻlsa ham."
      "U maktabda ishlaydi." "Bizning ikki farzandimiz bor.",

    expl ids.kc.pronoun_object "pronoun-object" "en"
      "I call her every evening. This gift is for them. The object pronoun comes after the verb or preposition."
      "Please help us." "I like him.",
    expl ids.kc.pronoun_object "pronoun-object" "ru"
      "Я звоню ей каждый вечер. Этот подарок для них. Дополнение-местоимение стоит после глагола или предлога."
      "Пожалуйста, помоги нам." "Он мне нравится.",
    expl ids.kc.pronoun_object "pronoun-object" "uz"
      "Men unga har kechqurun qoʻngʻiroq qilaman. Bu sovgʻa ular uchun. Toʻldiruvchi olmoshi feʼl yoki predlogdan keyin turadi."
      "Iltimos, bizga yordam bering." "Men uni yoqtiraman.",

    expl ids.kc.possessive_adjectives "possessive-adjectives" "en"
      "This is my book. Her children are at school. The possessive adjective stands before the noun and stays the same with plural nouns."
      "Our house is small." "Their car is new.",
    expl ids.kc.possessive_adjectives "possessive-adjectives" "ru"
      "Это моя книга. Её дети в школе. Притяжательное прилагательное стоит перед существительным и не меняется при множественном числе."
      "Наш дом маленький." "Их машина новая.",
    expl ids.kc.possessive_adjectives "possessive-adjectives" "uz"
      "Bu mening kitobim. Uning bolalari maktabda. Egalik olmoshi ot oldida turadi va otning koʻplikda boʻlishi bilan oʻzgarmaydi."
      "Bizning uyimiz kichkina." "Ularning mashinasi yangi.",

    expl ids.kc.possessive_s "possessive-s" "en"
      "This is Amina's bag. The children's room is upstairs. Add 's to a person or animal; for things, use of: the door of the car."
      "My brother's phone is new." "The colour of the wall is white.",
    expl ids.kc.possessive_s "possessive-s" "ru"
      "Это сумка Амины. Детская комната наверху. К человеку или животному добавляем 's; для вещей используем of: дверь машины."
      "Телефон моего брата новый." "Цвет стены белый.",
    expl ids.kc.possessive_s "possessive-s" "uz"
      "Bu Aminaning sumkasi. Bolalarning xonasi yuqori qavatda. Odam yoki hayvonga 's qoʻshiladi; narsalar uchun of ishlatiladi: mashinaning eshigi."
      "Akamning telefoni yangi." "Devorning rangi oq.",

    expl ids.kc.plural_regular "plural-regular" "en"
      "One book, two books. One box, two boxes. Add -s; add -es after s, x, ch, sh; a consonant + y becomes -ies."
      "three cats" "five boxes",
    expl ids.kc.plural_regular "plural-regular" "ru"
      "Одна книга, две книги. Одна коробка, две коробки. Добавляем -s; после s, x, ch, sh — -es; согласная + y переходит в -ies."
      "три кошки" "пять коробок",
    expl ids.kc.plural_regular "plural-regular" "uz"
      "Bitta kitob, ikkita kitob. Bitta quti, ikkita quti. -s qoʻshiladi; s, x, ch, sh dan keyin -es; undosh + y -ies ga aylanadi."
      "uchta mushuk" "beshta quti",

    expl ids.kc.plural_irregular "plural-irregular" "en"
      "One man, two men. One child, two children. Some plurals do not take -s; learn them as pairs."
      "my feet" "three people",
    expl ids.kc.plural_irregular "plural-irregular" "ru"
      "Один мужчина, двое мужчин. Один ребёнок, двое детей. Некоторые существительные образуют множественное число не через -s; учите их парами."
      "мои ноги" "три человека",
    expl ids.kc.plural_irregular "plural-irregular" "uz"
      "Bitta erkak, ikkita erkak. Bitta bola, ikkita bola. Baʼzi otlar koʻplikda -s olmaydi; ularni juft holda yodlang."
      "mening oyoqlarim" "uchta odam",

    expl ids.kc.demonstratives "demonstratives" "en"
      "This book is new. These books are old. Use this/that with one thing and these/those with more than one."
      "That house is big." "Those apples are sweet.",
    expl ids.kc.demonstratives "demonstratives" "ru"
      "Эта книга новая. Эти книги старые. This/that — с одним предметом, these/those — с несколькими."
      "Тот дом большой." "Те яблоки сладкие.",
    expl ids.kc.demonstratives "demonstratives" "uz"
      "Bu kitob yangi. Bu kitoblar eski. this/that bitta narsa bilan, these/those bir nechta narsa bilan ishlatiladi."
      "Anavi uy katta." "Ana u olmalar shirin.",
  ],
} | s.File
