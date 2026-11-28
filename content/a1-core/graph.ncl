# CEFR A1 core: the knowledge-component graph.
# `requires` points from a new KC into its prerequisite; nothing here points
# out of an existing live Articles KC, so their closures and statuses do not
# change. No `supports` edges (the redundant-supports check forbids a support
# already implied by a requires path).
let s = import "../lib/schema.ncl" in
let ids = import "lib/ids.ncl" in
let k = ids.kc in
let requires = fun src dst => { kind = "requires", from = src, to = dst } in
let contrasts = fun src dst => { kind = "contrasts", from = src, to = dst } in
{
  edges =
    [
      requires k.pronoun_object k.pronoun_subject,
      requires k.possessive_adjectives k.pronoun_subject,
      requires k.possessive_s k.plural_regular,
      requires k.plural_regular "en.gram.noun.countability",
      requires k.plural_irregular k.plural_regular,
      requires k.demonstratives k.plural_regular,
      requires k.be_affirmative k.pronoun_subject,
      requires k.be_negative k.be_affirmative,
      requires k.be_questions k.be_affirmative,
      requires k.there_is_are k.be_affirmative,
      requires k.present_affirmative k.pronoun_subject,
      requires k.present_third_person k.present_affirmative,
      requires k.present_negative k.present_affirmative,
      requires k.present_questions k.present_affirmative,
      requires k.continuous_affirmative k.be_affirmative,
      requires k.continuous_questions k.continuous_affirmative,
      requires k.can_ability k.pronoun_subject,
      requires k.can_requests k.can_ability,
      requires k.imperative k.present_affirmative,
      requires k.question_words k.be_questions,
      requires k.question_words k.present_questions,
      requires k.adjective_position k.be_affirmative,
      requires k.adverb_frequency k.present_affirmative,
      requires k.preposition_place k.be_affirmative,
      requires k.preposition_time k.present_affirmative,
      requires k.some_any "en.gram.noun.countability",
      requires k.much_many k.some_any,
    ]
    @ [
      contrasts k.present_affirmative k.continuous_affirmative,
      contrasts k.preposition_place k.preposition_time,
    ],
} | s.File
