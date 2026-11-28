import { PrototypeContent } from "../prototype/content";

const articleKc = (
  id: string,
  en: string,
  rank = 1,
  layout = { x: 500, y: 500 },
) => ({
  kc: {
    id,
    subject: "en",
    domain: "grammar",
    title: { en, ru: en, uz: en },
    review: {
      en: { status: "reviewed" },
      ru: { status: "reviewed" },
      uz: { status: "reviewed" },
    },
    level: rank === 1 ? "A1" : "A2",
    importance: 0.9,
    rule: en,
    tags: [],
    observed_only: false,
  },
  status: "live",
  level_rank: rank,
  layout_x: layout.x,
  layout_y: layout.y,
});

export function testContent(
  layout = { x: 500, y: 500 },
): PrototypeContent {
  const kcId = "en.gram.article.a-an-sound";
  const bundle = {
    format_version: 10,
    content_version: "test-content",
    content: {
      kcs: [articleKc(kcId, "a or an", 1, layout)],
      edges: [],
      lexemes: [
        {
          id: "en.lex.cat",
          lemma: "cat",
          pos: "noun",
          frequency_rank: 420,
          level: "A1",
          forms: ["cats"],
          kc_ids: [kcId],
          source: "test",
          license: "original",
          review: {
            en: { status: "reviewed" },
            ru: { status: "reviewed" },
            uz: { status: "reviewed" },
          },
          senses: [
            {
              id: "en.lex.cat.animal",
              definition_en: "a small animal with soft fur",
              gloss_uz: "mushuk",
              gloss_ru: "кошка",
              examples: [{ text: "I have a cat." }],
              collocations: ["a black cat", "feed the cat"],
              ipa_uk: "kæt",
              ipa_us: "kæt",
              review: {
                en: { status: "reviewed" },
                ru: { status: "reviewed" },
                uz: { status: "reviewed" },
              },
            },
          ],
        },
      ],
      item_lexemes: { "test.gap": ["en.lex.cat.animal"] },
      misconceptions: [
        {
          id: "en.mis.article.omission-l1",
          title: { en: "Article omitted", ru: "Артикль пропущен", uz: "Artikl tushib qolgan" },
          review: { en: { status: "reviewed" } },
          affects: [kcId],
          origin: "l1-transfer",
          l1: { uz: true, ru: true },
          prior: { uz: 0.4, ru: 0.5 },
          remedy: kcId,
          refutations: [],
        },
      ],
      items: [
        {
          session_eligible: true,
          item: {
            id: "test.gap",
            version: 1,
            subject: "en",
            family: "test.family",
            instruction: "instr.test",
            difficulty: 1,
            expected_time: 10,
            status: "live",
            breaks_calibration: false,
            review: {
              en: { status: "reviewed" },
              ru: { status: "reviewed" },
              uz: { status: "reviewed" },
            },
            type: "gap_fill",
            stimulus: { text: "I have ___ cat." },
            slots: [
              {
                kcs: [{ kc: kcId, role: "primary" }],
                answer: "a",
                accepted: ["a"],
                options: [],
                expected_wrong: [],
                hint: { en: "Listen to the first sound." },
                why: { en: "A cat starts with a consonant sound." },
              },
            ],
            case_sensitive: false,
            contractions: "strict",
            punctuation_sensitive: false,
          },
        },
      ],
      explanations: [
        {
          id: "test.explanation",
          target: { kind: "kc", id: kcId },
          depth: "short",
          lang: "en",
          body: "Use *a* before a consonant sound.",
          examples: [{ text: "I have a cat." }],
          review: { status: "reviewed" },
        },
        {
          id: "test.explanation.why",
          target: { kind: "kc", id: kcId },
          depth: "why",
          lang: "en",
          body: "The sound decides the article, not the letter.",
          l1_contrast: "Uzbek has no article; listen for the first sound.",
          examples: [],
          review: { status: "reviewed" },
        },
        {
          id: "test.trap.explanation",
          target: { kind: "misconception", id: "en.mis.article.omission-l1" },
          depth: "short",
          lang: "en",
          body: "Say the article even when your first language skips it.",
          examples: [],
          review: { status: "reviewed" },
        },
      ],
      instructions: [
        {
          id: "instr.test",
          text: { en: "Fill the gap." },
          review: { en: { status: "reviewed" } },
        },
      ],
    },
  };
  return new PrototypeContent(
    JSON.stringify(bundle),
    "test-content",
    "test-digest",
    "",
  );
}
