# Authoring contracts for content files. They catch typos and wrong shapes
# while writing; the refl-core Rust types are the source of truth, and
# content-build checks every file against them (docs/02-layer1-content.md).
let OneOf = fun values => std.contract.from_predicate (fun v => std.array.elem v values) in
let Unit = std.contract.from_predicate (fun v => std.is_number v && v >= 0 && v <= 1) in
let Localized = { uz | String | optional, ru | String | optional, en | String | optional } in
let Lang = OneOf ["uz", "ru", "en"] in

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
  },
  Kc = {
    id | String,
    subject | String,
    # From the subject's list of domains.
    domain | String,
    title | Localized,
    # A level key of the subject's scale.
    level | String,
    importance | Unit,
    rule | String,
    tags | Array String | optional,
  },
  Edge = {
    kind | OneOf ["requires", "supports", "contrasts", "part_of"],
    from | String,
    to | String,
  },
  Misconception = {
    id | String,
    title | Localized,
    affects | Array String,
    origin | OneOf ["l1-transfer", "overgeneralization", "form-confusion", "spelling-based"],
    l1 | { uz | Bool, ru | Bool },
    prior | { uz | Unit, ru | Unit },
    remedy | String,
  },
  Instruction = { id | String, text | Localized },
  Example = { text | String, audio | String | optional },
  Explanation = {
    id | String,
    target | { kind | OneOf ["kc", "misconception"], id | String },
    depth | OneOf ["short", "why", "deep"],
    lang | Lang,
    body | String,
    l1_contrast | String | optional,
    examples | Array Example | optional,
    review | { status | OneOf ["draft", "reviewed"], by | String | optional },
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
  Item = {
    id | String,
    version | Number,
    family | String,
    type | OneOf ["choice", "find_error", "matching", "true_false_not_given", "gap_fill", "transformation", "sentence_assembly", "translation", "dictation", "own_sentence", "writing", "shadowing"],
    instruction | String,
    stimulus | { text | String, audio | String | optional, image | String | optional },
    slots | Array Slot,
    difficulty | OneOf [1, 2, 3, 4, 5],
    expected_time | Number,
    topic | String | optional,
    status | OneOf ["draft", "review", "live", "retired"],
    case_sensitive | Bool | optional,
    contractions | OneOf ["strict", "equivalent"] | optional,
    punctuation_sensitive | Bool | optional,
    source | String | optional,
    license | String | optional,
    breaks_calibration | Bool | optional,
  },
  # What every content file exports. Lessons are checked by content-build only.
  File = {
    subjects | Array Subject | optional,
    kcs | Array Kc | optional,
    edges | Array Edge | optional,
    misconceptions | Array Misconception | optional,
    explanations | Array Explanation | optional,
    instructions | Array Instruction | optional,
    items | Array Item | optional,
    lessons | Array Dyn | optional,
  },
}
