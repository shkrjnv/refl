# CEFR A1 core: short explanations for continuous, modals, imperatives and questions.
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
    expl ids.kc.continuous_affirmative "continuous-affirmative" "en"
      "I am reading now. She is cooking dinner. Use am/is/are + verb-ing for an action happening now."
      "They are playing outside." "It is raining.",
    expl ids.kc.continuous_affirmative "continuous-affirmative" "ru"
      "Я сейчас читаю. Она готовит ужин. Для действия, которое происходит сейчас, нужно am/is/are + глагол с -ing."
      "Они играют на улице." "Идёт дождь.",
    expl ids.kc.continuous_affirmative "continuous-affirmative" "uz"
      "Men hozir oʻqiyapman. U kechki ovqat tayyorlayapti. Hozir boʻlayotgan ish-harakat uchun am/is/are + feʼlning -ing shakli ishlatiladi."
      "Ular tashqarida oʻynashyapti." "Yomgʻir yogʻyapti.",

    expl ids.kc.continuous_questions "continuous-questions" "en"
      "Are you listening? Is he working? Put am/is/are before the subject and keep the -ing form."
      "Is she sleeping?" "Are they coming?",
    expl ids.kc.continuous_questions "continuous-questions" "ru"
      "Ты слушаешь? Он работает? am/is/are ставится перед подлежащим, форма с -ing сохраняется."
      "Она спит?" "Они придут?",
    expl ids.kc.continuous_questions "continuous-questions" "uz"
      "Tinglayapsizmi? U ishlayaptimi? am/is/are egadan oldin turadi, -ing shakli saqlanadi."
      "U uxlayaptimi?" "Ular kelyaptimi?",

    expl ids.kc.can_ability "can-ability" "en"
      "I can swim. She can speak English. Use can + base verb for every subject; no -s and no do."
      "They can help you." "He can drive.",
    expl ids.kc.can_ability "can-ability" "ru"
      "Я умею плавать. Она говорит по-английски. can + глагол в начальной форме для всех лиц; без -s и без do."
      "Они могут помочь." "Он умеет водить.",
    expl ids.kc.can_ability "can-ability" "uz"
      "Men suzishni bilaman. U ingliz tilida gapira oladi. Barcha shaxslar uchun can + feʼlning asosiy shakli; -s ham, do ham yoʻq."
      "Ular yordam bera oladi." "U mashina boshqara oladi.",

    expl ids.kc.can_requests "can-requests" "en"
      "Can I open the window? Can you help me? Use Can I/you + base verb to ask for something or permission."
      "Can I have some water?" "Can you repeat that?",
    expl ids.kc.can_requests "can-requests" "ru"
      "Можно открыть окно? Ты можешь мне помочь? Can I/you + глагол в начальной форме — просьба или разрешение."
      "Можно воды?" "Повтори, пожалуйста.",
    expl ids.kc.can_requests "can-requests" "uz"
      "Derazani ochsam maylimi? Menga yordam bera olasizmi? Can I/you + feʼlning asosiy shakli iltimos yoki ruxsat uchun ishlatiladi."
      "Suv olsam maylimi?" "Buni qayta aytib bera olasizmi?",

    expl ids.kc.imperative "imperative" "en"
      "Open the door. Please sit down. Use the base verb without a subject; for a negative, say Don't + verb."
      "Turn left." "Don't touch the wall.",
    expl ids.kc.imperative "imperative" "ru"
      "Открой дверь. Садитесь, пожалуйста. Глагол в начальной форме без подлежащего; отрицание — Don't + глагол."
      "Поверни налево." "Не трогай стену.",
    expl ids.kc.imperative "imperative" "uz"
      "Eshikni oching. Iltimos, oʻtiring. Ega qoʻshilmaydi, feʼlning asosiy shakli ishlatiladi; inkor uchun Don't + feʼl."
      "Chapga buriling." "Devorga tegmang.",

    expl ids.kc.question_words "question-words" "en"
      "Where do you live? What is your name? The question word comes first and asks for the information you need."
      "When does the lesson start?" "Who is your teacher?",
    expl ids.kc.question_words "question-words" "ru"
      "Где ты живёшь? Как тебя зовут? Вопросительное слово стоит первым и показывает, какая информация нужна."
      "Когда начинается урок?" "Кто твой учитель?",
    expl ids.kc.question_words "question-words" "uz"
      "Qayerda yashaysiz? Ismingiz nima? Savol soʻzi birinchi turadi va qanday maʼlumot kerakligini koʻrsatadi."
      "Dars qachon boshlanadi?" "Sizning oʻqituvchingiz kim?",
  ],
} | s.File
