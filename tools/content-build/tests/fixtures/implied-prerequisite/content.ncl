# Every session-eligible item of live KC k.a has k.b as a secondary KC, and
# k.a does not require k.b. The base edge k.b requires k.a is left out: with
# it, k.b would be downstream of k.a, which is `secondary-downstream`.
# reviews.lock marks everything reviewed, so the items are session-eligible.
# Fails with `implied-prerequisite` only.
let b = import "../lib/base.ncl" in
let a = b.live_kc "en.k.a" in
{
  subjects = b.subjects,
  kcs = b.kcs,
  edges = [],
  misconceptions = b.misconceptions,
  instructions = b.instructions,
  explanations = b.explanations @ a.explanations,
  items =
    std.array.generate
      (fun i =>
        b.gap {
          id = "it.en.k.a.%{std.to_string i}",
          family = "en.f.en.k.a.%{std.to_string (i % 3)}",
          difficulty = 1 + i % 2,
          slot_kcs = [
            { kc = "en.k.a", role = "primary" },
            { kc = "en.k.b", role = "secondary" },
          ],
        }
      )
      10,
}
