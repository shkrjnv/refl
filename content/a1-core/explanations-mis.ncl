# CEFR A1 core: micro-explanations for the L1 misconceptions.
# Shown right after an error; one correction and one reason, at most two
# sentences (docs/02, «Заблуждение»).
let s = import "../lib/schema.ncl" in
let ids = import "lib/ids.ncl" in
let micro = fun mis_id name lang_code text =>
  {
    id = "expl.a1.mis.%{name}.short.%{lang_code}",
    target = { kind = "misconception", id = mis_id },
    depth = "short",
    lang = lang_code,
    body = text,
    review = { status = "draft" },
  }
in
{
  explanations = [
    micro ids.mis.drop_copula "drop-copula" "en"
      "English needs be: not *He a student*, but *He is a student*. Russian and Uzbek can leave it out, English cannot.",
    micro ids.mis.drop_copula "drop-copula" "ru"
      "В английском нужен be: не *He a student*, а *He is a student*. В русском глагол в настоящем времени опускается, в английском нет.",
    micro ids.mis.drop_copula "drop-copula" "uz"
      "Inglizchada be kerak: *He a student* emas, *He is a student*. Oʻzbekchada feʼl tushib qolishi mumkin, inglizchada yoʻq.",

    micro ids.mis.am_agree "am-agree" "en"
      "Agree is a verb, so it does not need am: *I agree*, not *I am agree*. Russian says «я согласен», but English uses one verb.",
    micro ids.mis.am_agree "am-agree" "ru"
      "Agree — глагол, поэтому am не нужен: *I agree*, а не *I am agree*. По-русски «я согласен», по-английски — один глагол.",
    micro ids.mis.am_agree "am-agree" "uz"
      "Agree — feʼl, shuning uchun am kerak emas: *I agree*, *I am agree* emas. Oʻzbekchada ham «men roziman» bitta feʼl bilan aytiladi.",

    micro ids.mis.drop_subject "drop-subject" "en"
      "English needs the subject pronoun: *She is my sister*, not *Is my sister*. The verb cannot stand alone like in Russian or Uzbek.",
    micro ids.mis.drop_subject "drop-subject" "ru"
      "В английском нужно подлежащее-местоимение: *She is my sister*, а не *Is my sister*. Глагол не стоит один, как в русском.",
    micro ids.mis.drop_subject "drop-subject" "uz"
      "Inglizchada ega olmoshi kerak: *She is my sister*, *Is my sister* emas. Oʻzbekchada ega tushib qolishi mumkin, inglizchada yoʻq.",

    micro ids.mis.third_person_s "third-person-s" "en"
      "With he, she, it the verb takes -s: *He goes*, not *He go*. The -s is the only change in the present simple.",
    micro ids.mis.third_person_s "third-person-s" "ru"
      "С he, she, it у глагола появляется -s: *He goes*, а не *He go*. В Present Simple это единственное изменение.",
    micro ids.mis.third_person_s "third-person-s" "uz"
      "he, she, it bilan feʼlga -s qoʻshiladi: *He goes*, *He go* emas. Present Simple da faqat shu oʻzgaradi.",

    micro ids.mis.no_do "no-do" "en"
      "Questions and negatives need do/does: *Does he work?*, *He doesn't work*. Without do, English has no question or negative.",
    micro ids.mis.no_do "no-do" "ru"
      "В вопросах и отрицаниях нужно do/does: *Does he work?*, *He doesn't work*. Без do английский вопрос и отрицание не строит.",
    micro ids.mis.no_do "no-do" "uz"
      "Savol va inkorlarda do/does kerak: *Does he work?*, *He doesn't work*. do boʻlmasa, inglizchada savol va inkor qurilmaydi.",

    micro ids.mis.word_order "word-order" "en"
      "After a question word, put the auxiliary before the subject: *Where do you live?*, not *Where you live?*.",
    micro ids.mis.word_order "word-order" "ru"
      "После вопросительного слова вспомогательный глагол стоит перед подлежащим: *Where do you live?*, а не *Where you live?*.",
    micro ids.mis.word_order "word-order" "uz"
      "Savol soʻzidan keyin yordamchi feʼl egadan oldin turadi: *Where do you live?*, *Where you live?* emas.",

    micro ids.mis.time_preposition "time-preposition" "en"
      "Use in for months and years, on for days, at for clock times: *in July*, *on Monday*, *at seven*.",
    micro ids.mis.time_preposition "time-preposition" "ru"
      "In — месяцы и годы, on — дни, at — время на часах: *in July*, *on Monday*, *at seven*.",
    micro ids.mis.time_preposition "time-preposition" "uz"
      "In — oylar va yillar, on — kunlar, at — soat vaqti: *in July*, *on Monday*, *at seven*.",

    micro ids.mis.much_many "much-many" "en"
      "Use many with countable plurals and much with uncountable nouns: *many books*, *much water*.",
    micro ids.mis.much_many "much-many" "ru"
      "Many — с исчисляемыми во множественном числе, much — с неисчисляемыми: *many books*, *much water*.",
    micro ids.mis.much_many "much-many" "uz"
      "Many — sanaladigan koʻplik bilan, much — sanalmaydigan otlar bilan: *many books*, *much water*.",
  ],
} | s.File
