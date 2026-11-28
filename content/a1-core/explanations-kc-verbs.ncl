# CEFR A1 core: short explanations for be, there is/are and the present simple.
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
    expl ids.kc.be_affirmative "be-affirmative" "en"
      "I am a teacher. She is at home. They are happy. Use am with I, is with he/she/it, are with you/we/they."
      "We are ready." "It is cold.",
    expl ids.kc.be_affirmative "be-affirmative" "ru"
      "Я учитель. Она дома. Они счастливы. С I — am, с he/she/it — is, с you/we/they — are."
      "Мы готовы." "Холодно.",
    expl ids.kc.be_affirmative "be-affirmative" "uz"
      "Men oʻqituvchiman. U uyda. Ular xursand. I bilan am, he/she/it bilan is, you/we/they bilan are ishlatiladi."
      "Biz tayyormiz." "Havo sovuq.",

    expl ids.kc.be_negative "be-negative" "en"
      "I am not tired. He isn't at work. They aren't late. Put not after am/is/are."
      "She is not here." "We aren't ready.",
    expl ids.kc.be_negative "be-negative" "ru"
      "Я не устал. Он не на работе. Они не опоздали. not ставится после am/is/are."
      "Её здесь нет." "Мы не готовы.",
    expl ids.kc.be_negative "be-negative" "uz"
      "Men charchamadim. U ishda emas. Ular kechikishmadi. not am/is/are dan keyin turadi."
      "U bu yerda emas." "Biz tayyor emasmiz.",

    expl ids.kc.be_questions "be-questions" "en"
      "Are you ready? Is she at home? Put am/is/are before the subject and answer with the same form."
      "Is he your brother?" "Are they at school?",
    expl ids.kc.be_questions "be-questions" "ru"
      "Ты готов? Она дома? am/is/are ставится перед подлежащим, а ответ повторяет ту же форму."
      "Он твой брат?" "Они в школе?",
    expl ids.kc.be_questions "be-questions" "uz"
      "Tayyormisiz? U uydami? am/is/are egadan oldin turadi, javobda ham shu shakl qaytariladi."
      "U sizning ukangizmi?" "Ular maktabdami?",

    expl ids.kc.there_is_are "there-is-are" "en"
      "There is a cat on the sofa. There are two cats in the garden. Use is with one thing, are with many."
      "There is a shop near my house." "There are five books on the table.",
    expl ids.kc.there_is_are "there-is-are" "ru"
      "На диване кошка. В саду две кошки. С одним предметом — there is, с несколькими — there are."
      "Рядом с домом есть магазин." "На столе пять книг.",
    expl ids.kc.there_is_are "there-is-are" "uz"
      "Divanda mushuk bor. Bogʻda ikkita mushuk bor. Bitta narsa bilan there is, bir nechta bilan there are ishlatiladi."
      "Uyim yonida doʻkon bor." "Stolda beshta kitob bor.",

    expl ids.kc.present_affirmative "present-affirmative" "en"
      "I work in a school. They live in Samarkand. With I/you/we/they the verb has no ending."
      "We play football on Sunday." "You speak English well.",
    expl ids.kc.present_affirmative "present-affirmative" "ru"
      "Я работаю в школе. Они живут в Самарканде. С I/you/we/they у глагола нет окончания."
      "По воскресеньям мы играем в футбол." "Ты хорошо говоришь по-английски.",
    expl ids.kc.present_affirmative "present-affirmative" "uz"
      "Men maktabda ishlayman. Ular Samarqandda yashaydi. I/you/we/they bilan feʼl oxiri qoʻshimcha olmaydi."
      "Yakshanba kuni futbol oʻynaymiz." "Siz ingliz tilida yaxshi gapirasiz.",

    expl ids.kc.present_third_person "present-third-person" "en"
      "He works in a bank. She goes to school. With he/she/it add -s, or -es after ch, sh, s, x."
      "My sister watches films." "It rains a lot here.",
    expl ids.kc.present_third_person "present-third-person" "ru"
      "Он работает в банке. Она ходит в школу. С he/she/it к глаголу добавляется -s, а после ch, sh, s, x — -es."
      "Моя сестра смотрит фильмы." "Здесь часто идёт дождь.",
    expl ids.kc.present_third_person "present-third-person" "uz"
      "U bankda ishlaydi. U maktabga boradi. he/she/it bilan feʼlga -s, ch, sh, s, x dan keyin -es qoʻshiladi."
      "Singlim film tomosha qiladi." "Bu yerda koʻp yomgʻir yogʻadi.",

    expl ids.kc.present_negative "present-negative" "en"
      "I don't like coffee. She doesn't live here. Use don't with I/you/we/they, doesn't with he/she/it, then the base verb."
      "They don't work on Sunday." "He doesn't speak Russian.",
    expl ids.kc.present_negative "present-negative" "ru"
      "Я не люблю кофе. Она здесь не живёт. С I/you/we/they — don't, с he/she/it — doesn't, дальше глагол в начальной форме."
      "По воскресеньям они не работают." "Он не говорит по-русски.",
    expl ids.kc.present_negative "present-negative" "uz"
      "Men qahva yoqtirmayman. U bu yerda yashamaydi. I/you/we/they bilan don't, he/she/it bilan doesn't, keyin feʼlning asosiy shakli."
      "Yakshanba kuni ular ishlamaydi." "U rus tilida gapirmaydi.",

    expl ids.kc.present_questions "present-questions" "en"
      "Do you like tea? Does she work here? Start with Do/Does, then the subject, then the base verb."
      "Do they live in Tashkent?" "Does he play tennis?",
    expl ids.kc.present_questions "present-questions" "ru"
      "Ты любишь чай? Она здесь работает? Сначала Do/Does, потом подлежащее, потом глагол в начальной форме."
      "Они живут в Ташкенте?" "Он играет в теннис?",
    expl ids.kc.present_questions "present-questions" "uz"
      "Choy yoqtirasizmi? U bu yerda ishlaydimi? Avval Do/Does, keyin ega, keyin feʼlning asosiy shakli keladi."
      "Ular Toshkentda yashaydimi?" "U tennis oʻynaydimi?",
  ],
} | s.File
