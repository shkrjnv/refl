# D3 and D1 follow-up evidence

Branch: `review/d3-screenshots`, based on the D3 code branch `fix/d3-today-photo`.

Playwright captures in Chromium at 360×800 and 1440×900, `cream-paper`
(light) and `deep-umber` (dark). The reader word card uses the reader themes
`paper` (light) and `midnight` (dark). Locale is `ru`.

`before/` is `bc57455` (the D1 merge); `after/` is the D3 branch HEAD without
the screenshot commit. The file names match across both folders.

| Folder | What it shows |
| --- | --- |
| `01-hero-photo` | Today hero on the responsive AVIF/WebP/JPEG candidate set. |
| `02-reader-word` | Word card with a separate hint, labelled Example quote, and field label. |
| `03-more-scrim` | Wide bottom dock: More popover with the theme scrim; phone width is unchanged. |

`asset-budget.txt` records the candidate sizes, the budget asserted by
`web/src/assets/hero.test.ts`, and the production build output.
