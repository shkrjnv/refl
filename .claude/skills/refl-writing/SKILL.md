---
name: refl-writing
description: Use for any text refl shows or publishes — explanations, micro-explanations, task instructions, hints, "why this task", feedback, interface microcopy, Iella and Sendo lines, onboarding, notifications, the README and release notes. Makes them methodical, concrete and human, not AI-sounding.
---

# Writing for refl

refl teaches. Every sentence a learner reads either helps them understand something or costs them attention. Write like a good teacher who respects the reader's time: concrete, warm, exact, short. Never like a chatbot or an ad.

This skill covers three registers:

1. **Learner-facing** (explanations, feedback, hints, instructions, microcopy, companions, onboarding, notifications) in uz / ru / en.
2. **Mentor-facing** (summaries, reports): calm, factual, actionable.
3. **Public / developer** (README, release notes, docs): plain technical English or Russian.

Before writing, know three things: **who reads it** (age band, level, native language), **what they must be able to do after reading**, and **where it appears** (a button, a feedback line, a full explanation). If you can't answer, ask or check the spec (docs/02, docs/06, docs/12).

## Core principles

1. **Example first, rule second.** Show the thing working, then name the pattern. "I have **a** cat. **The** cat is black." before any rule about first mention.
2. **One idea per sentence, one point per text.** A short explanation explains one rule. If you need "also", it's two texts.
3. **Concrete beats abstract.** Real nouns, real numbers, real situations from the learner's world: Tashkent, school, football, games, family, a bus, plov. Not "an object", "a situation", "various contexts".
4. **Say why, briefly.** A rule with a reason sticks: "the listener already knows which one" explains *the* better than "definite article".
5. **Contrast is the strongest tool.** Two sentences that differ in one word and differ in meaning teach more than a paragraph. Use it for misconceptions and confusable topics (docs/12, "Refutation", "Variation").
6. **Native-language contrast is written, not translated.** Uzbek through *bir*, *-ni*, word order; Russian through what Russian lacks. Each is its own text (docs/02).
7. **Respect the reader.** No talking down to children, no flattery for adults. No "Great job!" after an easy item; specific acknowledgement only when earned ("You fixed the article error you made last week").
8. **Errors are information, not failure.** Feedback names what happened and what to do: "Here *the*: the cat was already mentioned." Never "Wrong!", never "Oops", never shame.
9. **Numbers are honest.** Always with what they mean and how sure we are: "B1 · 40% of the way to B2 (likely 25–55%)". Never round up to look better.
10. **Shorter.** Cut every word that doesn't change meaning. Then cut once more.

## Lengths

| Text | Limit |
| --- | --- |
| button, tab, label | 1–3 words, a verb for actions ("Start", "Check", "Next") |
| task instruction | one sentence, imperative, ≤ 12 words |
| micro-explanation after an error | ≤ 2 sentences: what's right here + why |
| hint | one nudge toward the rule, never the answer |
| "why this task" | one sentence with a real number from the model |
| short explanation (depth short) | 2–4 sentences + 2 examples |
| "why" explanation | ≤ 120 words, one contrast pair |
| deep explanation | structured: examples → pattern → edge cases → contrast with L1; components (table, timeline, decision tree) instead of long prose |
| notification | ≤ 90 characters, says what and why now, no guilt |
| companion line | ≤ 2 short sentences, in character |

## What makes text sound like AI — never do this

**Openers and fillers:** "Let's dive in", "Let's explore", "In this lesson we will…", "It's important to note that", "It's worth mentioning", "Simply put", "In other words", "Basically". Ru: «Давайте разберёмся», «Давайте погрузимся», «Важно отметить, что», «Стоит отметить», «Проще говоря», «Иными словами», «В данном уроке мы рассмотрим». Uz: «Keling, chuqurroq oʻrganamiz», «Shuni taʼkidlash joizki».

**Hype and empty adjectives:** "powerful", "seamless", "unlock your potential", "journey", "master like a pro", "game-changer", "magic", «мощный», «уникальный», «невероятный», «прокачай», «путешествие в мир английского».

**Rhetorical tics:**
- "It's not just X, it's Y" / «Это не просто X, а Y» — at most once per document, preferably never.
- Triplets by reflex: "fast, simple and effective". Use a list of three only when there are exactly three real things.
- Questions answered immediately: "Why does this matter? Because…".
- Colon-reveals and dramatic dashes: "The secret? Practice." Keep dashes for real breaks; in Russian use «—» where grammar requires it, not for drama.
- Ending with a moral or a summary of what was just said ("In conclusion…", «Таким образом…»).

**Fake warmth:** "Great question!", "You've got this!", "Don't worry!", «Не переживай!», «Ты справишься!», excessive emoji. Iella can be warm, but warmth comes from specifics, not exclamation marks.

**Vagueness:** "in various situations", "in many cases", "plays an important role", «в различных ситуациях», «играет важную роль». Replace with the actual case.

**Over-hedging:** "may potentially help to some extent". Say what is true; state uncertainty once, with a number if we have one.

**Translation smell:** Russian or Uzbek that reads like English (calques, passive everywhere, "является", «осуществлять», «данный», «в рамках»). Write each language natively. Uzbek in Latin script with oʻ, gʻ (U+02BB), never o', o` or Cyrillic unless the setting says so.

## Patterns that work

**Short explanation (KC):**
> I have **a** cat. **The** cat is black.
> The first time we mention a thing, we say *a*: the listener doesn't know it yet. After that it's *the*: now they know which one.

**Micro-explanation (misconception), ≤ 2 sentences:**
> Before a single countable thing you need an article: not *I have cat*, but *I have **a** cat*.
> Ru: «Перед одним исчисляемым предметом нужен артикль: не *I have cat*, а *I have **a** cat*. В русском его нет, поэтому его легко пропустить.»

**Contrast pair (refutation):**
> *I've lost my keys.* — they're still lost, I'm standing at the door.
> *I lost my keys.* — a story about the past; maybe I found them since.

**Feedback after an error:** state the correct form, the one reason, nothing else.
> *the* — the cat was already mentioned in the first sentence.

**"Why this task":** factor + number from the model.
> Reviewing *since / for*: without a review, in 3 days you'd likely forget it.
> You chose *a* instead of *the* 4 times this week; this task is exactly about that.

**Instruction:** verb + object + constraint.
> Fill the gap with *a*, *an* or *the*.
> Rewrite the sentence using *since*.

**Empty state:** what this place is for + the one action.
> Your error notebook is empty. Mistakes you make in sessions will appear here with their fixes.

**Return after a break:**
> It's been 23 days. Let's spend 8 minutes checking what stuck; recalling it now makes it last longer.

## Companions

- **Iella** (swallow, mail carrier): curious, light, a little playful, travels a lot, notices details. Speaks in short sentences, uses images from flight and letters sparingly (not in every line). Never scolds, never comments on missed days, never talks during an answer. Example: "This letter came from someone who mixes up *a* and *the*. Let's help them."
- **Sendo** (jerboa, library keeper): calm, precise, quietly funny, loves words and where they come from. Example: "*Since* marks the start point. *For* measures the length. Same clock, different questions."
- Companion lines follow the learner's level (i+1, docs/06): simpler English than the learner's level, native language when the learner is below A2.

## Age bands

| Band | How |
| --- | --- |
| 8–12 | concrete pictures and situations, shorter sentences, no grammar terms without an example next to them, the companion carries more of the voice |
| 13–17 | direct, no baby talk, no cringe slang; respect autonomy ("you can skip this"); humour dry, not forced |
| 18+ | efficient, terms allowed (Present Perfect, countable), more "why", exam context when relevant |

## README and docs

- Lead with what the thing is and does, in one or two sentences. No slogans in technical text.
- Prefer specifics over claims: "proven with an automaton product" over "rigorously verified".
- Each section answers one question a reader has. Tables for comparisons, lists only for parallel items, prose for reasoning.
- Mark what is built vs planned. Never describe a plan as a feature.
- Link to the spec instead of repeating it.

## Self-check before you finish

1. Read it aloud. Would a good teacher say this to this learner? If it sounds like a press release or a chatbot, rewrite.
2. Search your text for the phrases in "never do this" (all three languages).
3. Is there an example before the rule? A reason? Only one idea?
4. Count words against the length table; cut.
5. Every number: is it real, and is its uncertainty shown?
6. Each language reads as native, not as a translation. Uzbek and Russian versions go to the author for review (docs/02: `review = draft` until reviewed).
