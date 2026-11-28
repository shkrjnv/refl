# CEFR A1 core: shared builders for the starter bank.
# Every item in this unit is a draft: a person has not reviewed its `why`
# text yet, so it is never shown to a learner (docs/02, «Вычитка и показ»).
let ids = import "ids.ncl" in
let mk_primary = fun id => { kc = id, role = "primary" } in
let mk_item = fun kind r =>
  {
    id = r.id,
    version = if std.record.has_field "version" r then r.version else 1,
    subject = "en",
    family = r.family,
    type = kind,
    instruction = r.instruction,
    difficulty = r.difficulty,
    expected_time = r.expected_time,
    status = "draft",
    stimulus = { text = r.stimulus },
    slots = [r.slot],
  }
in
{
  kc = ids.kc,
  mis = ids.mis,

  primary = mk_primary,
  secondary = fun id => { kc = id, role = "secondary" },

  item = mk_item,

  # One recognition task: the answer plus two wrong options.
  choice_item = fun r =>
    mk_item "choice" {
      id = "a1.%{r.slug}.choice",
      family = "en.a1.%{r.slug}.choice",
      instruction = "instr.choose-option",
      difficulty = r.difficulty,
      expected_time = if std.record.has_field "expected_time" r then r.expected_time else 10,
      stimulus = r.stimulus,
      slot = {
        kcs = [mk_primary r.kc],
        answer = r.answer,
        accepted = [r.answer],
        options = r.options,
        why = r.why,
      },
    },

  # One production task: the learner types the form; the error patterns name
  # the misconception the wrong form reveals.
  gap_item = fun r =>
    mk_item "gap_fill" {
      id = "a1.%{r.slug}.gap",
      family = "en.a1.%{r.slug}.gap",
      instruction = if std.record.has_field "instruction" r then r.instruction else "instr.gap-fill-form",
      difficulty = r.difficulty,
      expected_time = if std.record.has_field "expected_time" r then r.expected_time else 20,
      stimulus = r.stimulus,
      slot = {
        kcs = [mk_primary r.kc],
        answer = r.answer,
        accepted = if std.record.has_field "accepted" r then r.accepted else [r.answer],
        expected_wrong = r.expected_wrong,
        why = r.why,
      },
    },
}
