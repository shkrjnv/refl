# D1 UI defect evidence

Branch: `fix/ui-defects`. Baseline for every `before` image: `ba31ee5`.

Screenshots are Playwright captures in Chromium at 360×800 and 1440×900,
with `cream-paper` (light) and `deep-umber` (dark). The reader word card uses
the reader's own themes: `paper` for light and `midnight` for dark. Locale is
`ru` unless the file name says otherwise.

`before/` and `after/` contain the same file names, so each pair can be
compared directly.

| Folder | What it shows |
| --- | --- |
| `01-today-header` | Streak and rejected-sync cue; `ru` and `uz` long strings. |
| `02-username` | Display name on Today and Profile; account id and Copy button under Settings → Account. |
| `03-dock` | Bottom of Today, Vocabulary, Progress, and Profile at 360 px with the phone dock; Today and Vocabulary with the bottom navigation position at 1440 px. |
| `04-more-sheet` | More sheet at a phone width and above the wide bottom dock. |
| `05-reader-word` | Reader word card in light and dark reader themes. |
| `06-companions` | Companion cards, selected label, and the locked Sendo state. |
| `07-curve` | Forgetting curve with the day-axis label. |
| `08-text` | Text wrapping at 360 px in `ru` and `uz`, plus Today and Profile at 1440 px. |

## Text audit (item 8)

Found and fixed:

- Today, `ru`, 360 px: mastery titles clipped mid-word — “Без артикля: о предметах во…” and “Исчисляемые и неисчисляемые су…”.
- Today, `ru`, 360 px: error-notebook rows clipped — “Множественное число у неисчисляем…”.
- Today, `ru`, 360 px: private-note rows clipped — “Исчисляемые и неисчисляемые существител…”.
- Profile, `ru`, 360 px: the same error and quick-note titles were clipped.
- Progress, `ru`, 360 px: skill and misconception titles used a one-line ellipsis.
- Vocabulary, `ru`/`uz`, 360 px: the “Add your own word” heading and note were squeezed into a narrow column by the right-aligned button, leaving one- and two-word lines.
- Long display names in the Today and Profile headings now wrap instead of escaping the card.

Checked and left as designed:

- Book titles and book sources use a two-line clamp with an ellipsis.
- The vocabulary filter row and the settings tab row scroll horizontally on a phone.
- Reader toolbar titles and note tab titles use a one-line ellipsis in their fixed bars.
- The Notes explorer and inspector slide off-canvas below their breakpoints.
