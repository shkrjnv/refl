# Authoring contracts for content files. They catch typos and wrong shapes
# while writing; the refl-core Rust types are the source of truth, and
# content-build checks every file against them (docs/02-layer1-content.md).
let OneOf = fun values => std.contract.from_predicate (fun v => std.array.elem v values) in
let Unit = std.contract.from_predicate (fun v => std.is_number v && v >= 0 && v <= 1) in
let Localized = { uz | String | optional, ru | String | optional, en | String | optional } in
let Lang = OneOf ["uz", "ru", "en"] in
# Review per language; a language without a record is draft (docs/02,
# «Вычитка и показ»). `reviewed` needs `by`: the name of the reviewer.
# `hash` is the SHA-256 of the exact text the review covers; `reviewed` needs
# it, and a text edited after the review is the error `reviewed-text-changed`.
# `hashes.txt` from `report --review` has the current one.
let ReviewRecord = { status | OneOf ["draft", "reviewed"], by | String | optional, hash | String | optional } in
let LangReviews = { uz | ReviewRecord | optional, ru | ReviewRecord | optional, en | ReviewRecord | optional } in
let Representation = OneOf ["concrete", "pictorial", "abstract"] in
# A refutation is either a contrast pair (the disputed form in two sentences
# and the meaning difference) or a claim with a counterexample
# (docs/12-method.md, «Опровержение»).
let Refutation =
  std.contract.custom
    (
      fun label value =>
        if !(std.is_record value) || !(std.record.has_field "kind" value) then
          'Error { message = "a refutation needs a `kind`" }
        else if value.kind == "contrast" then
          'Ok
            (
              std.contract.apply
                {
                  kind | OneOf ["contrast"],
                  correct | String,
                  incorrect | String,
                  difference | Localized,
                }
                label
                value
            )
        else if value.kind == "counterexample" then
          'Ok
            (
              std.contract.apply
                {
                  kind | OneOf ["counterexample"],
                  claim | String,
                  counterexample | String,
                  explanation | Localized,
                }
                label
                value
            )
        else
          'Error {
            message = "unknown refutation kind `%{std.to_string value.kind}`",
          }
    )
in

{
  Subject = {
    id | String,
    title | Localized,
    scale | {
      levels | Array { key | String, title | Localized },
      step_logit | Number,
    },
    domains | Array String,
    checker | OneOf ["en-words"],
    # The orthography KC a typo updates.
    spelling_kc | String,
  },
  Kc = {
    id | String,
    subject | String,
    # From the subject's list of domains.
    domain | String,
    title | Localized,
    review | LangReviews | optional,
    # A level key of the subject's scale.
    level | String,
    importance | Unit,
    rule | String,
    tags | Array String | optional,
    # Only receives observations from other tasks (the subject's spelling
    # KC): never live, not listed as blocked.
    observed_only | Bool | optional,
  },
  Edge = {
    kind | OneOf ["requires", "supports", "contrasts", "part_of"],
    from | String,
    to | String,
  },
  LexemeExample = { text | String, audio_key | String | optional },
  LexemeSense = {
    id | String,
    definition_en | String,
    gloss_uz | String | optional,
    gloss_ru | String | optional,
    examples | Array LexemeExample | optional,
    collocations | Array String | optional,
    ipa_uk | String | optional,
    ipa_us | String | optional,
    audio_key | String | optional,
    review | LangReviews | optional,
  },
  Lexeme = {
    id | String,
    lemma | String,
    pos | OneOf ["noun", "verb", "adj", "adv", "prep", "pron", "det", "conj", "phrase"],
    frequency_rank | Number,
    level | String,
    forms | Array String | optional,
    senses | Array LexemeSense,
    kc_ids | Array String | optional,
    source | String | optional,
    license | String | optional,
    review | LangReviews | optional,
  },
  Misconception = {
    id | String,
    title | Localized,
    review | LangReviews | optional,
    affects | Array String,
    origin | OneOf ["l1-transfer", "overgeneralization", "form-confusion", "spelling-based"],
    l1 | { uz | Bool, ru | Bool },
    prior | { uz | Unit, ru | Unit },
    remedy | String,
    # Optional authored refutations; multiple entries are safe branches.
    refutations | Array Refutation | optional,
  },
  Instruction = {
    id | String,
    text | Localized,
    review | LangReviews | optional,
  },
  Example = { text | String, audio | String | optional, representation | Representation | optional },
  Explanation = {
    id | String,
    target | { kind | OneOf ["kc", "misconception"], id | String },
    depth | OneOf ["short", "why", "deep"],
    lang | Lang,
    body | String,
    l1_contrast | String | optional,
    examples | Array Example | optional,
    review | ReviewRecord,
  },
  Slot = {
    kcs | Array { kc | String, role | OneOf ["primary", "secondary"] },
    answer | String,
    accepted | Array String,
    options | Array { text | String, misconception | Dyn, confuses_with | String | optional } | optional,
    expected_wrong | Array { pattern | String, misconception | String, confuses_with | String | optional } | optional,
    hint | Localized | optional,
    why | Localized,
  },
  # The fields every item has (docs/10, «Вид задания: трейт и данные»).
  ItemHeader = {
    id | String,
    version | Number,
    subject | String,
    family | String,
    type | String,
    instruction | String,
    difficulty | OneOf [1, 2, 3, 4, 5],
    expected_time | Number,
    representation | Representation | optional,
    topic | String | optional,
    status | OneOf ["draft", "review", "live", "retired"],
    source | String | optional,
    license | String | optional,
    breaks_calibration | Bool | optional,
    # Review of the `why` and `hint` texts of all slots, per language.
    review | LangReviews | optional,
    # Items with the same group are reviewed together: one hash for all.
    review_group | String | optional,
  },
  ItemBody = {
    stimulus | { text | String, audio | String | optional, image | String | optional },
    slots | Array Slot,
  },
  # Settings of the English input checker; only typed-input kinds have them.
  EnglishCheck = {
    case_sensitive | Bool | optional,
    contractions | OneOf ["strict", "equivalent"] | optional,
    punctuation_sensitive | Bool | optional,
  },
  english_input_kinds = ["gap_fill", "transformation", "sentence_assembly", "translation", "dictation"],
  other_kinds = ["choice", "find_error", "matching", "true_false_not_given", "own_sentence", "writing", "shadowing"],
  # An item accepts exactly the fields of its kind, chosen by `type`: a
  # checker flag on a choice item is an error, not silently ignored.
  Item =
    std.contract.custom
      (
        fun label value =>
          if !(std.is_record value) || !(std.record.has_field "type" value) then
            'Error { message = "an item needs a `type`" }
          else
            let kind = value."type" in
            if std.array.elem kind english_input_kinds then
              'Ok (std.contract.apply (ItemHeader & ItemBody & EnglishCheck) label value)
            else if std.array.elem kind other_kinds then
              'Ok (std.contract.apply (ItemHeader & ItemBody) label value)
            else
              'Error { message = "unknown item type `%{std.to_string kind}`" }
      ),
  # What every content file exports. Lessons are checked by content-build only.
  File = {
    subjects | Array Subject | optional,
    kcs | Array Kc | optional,
    lexemes | Array Lexeme | optional,
    edges | Array Edge | optional,
    misconceptions | Array Misconception | optional,
    explanations | Array Explanation | optional,
    instructions | Array Instruction | optional,
    items | Array Item | optional,
    lessons | Array Dyn | optional,
  },
}
