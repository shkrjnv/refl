import { PrototypeContent } from "../prototype/content";

const articleKc = (id: string, en: string, rank = 1) => ({
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
});

export function testContent(): PrototypeContent {
  const kcId = "en.gram.article.a-an-sound";
  const bundle = {
    format_version: 10,
    content_version: "test-content",
    content: {
      kcs: [articleKc(kcId, "a or an", 1)],
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
