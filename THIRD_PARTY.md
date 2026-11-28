# Third-party data and models

Every dataset, dictionary, voice or model that refl uses but did not write is listed here before it is used: what it is, where it comes from, its license and what that license requires ([docs/14](docs/14-privacy-license.md), «Чужие данные и модели»).

## Fonts

| | |
| --- | --- |
| What | Geologica by Monokrom (variable weight), Pixelify Sans by the Pixelify Sans Project Authors, and a Noto Sans subset for IPA and the Uzbek `ʻ` (U+02BB) |
| Used for | the interface text (Geologica), pixel display accents (Pixelify Sans), and IPA transcriptions served under the Geologica family so a transcript never falls back to an arbitrary system font |
| Where from | Google Fonts / upstream projects; the exact `.woff2` files are committed under `web/public/fonts/`; no runtime CDN request is made |
| License | SIL Open Font License 1.1 for all three; full notices: [`licenses`](web/public/fonts/) in the client assets |
| What it requires | the copyright and OFL notices travel with the font files; a modified version may not use a Reserved Font Name. We do not modify the font programs; the Noto subset is served under the `Geologica` CSS family name only as a distinct IPA face |
| How we comply | the OFL text and attribution are shipped with the PWA; `web/src/theme/fonts.test.ts` checks glyph coverage and that no external font URL is referenced |

## SCOWL word lists

| | |
| --- | --- |
| What | Spell Checker Oriented Word Lists by Kevin Atkinson: the `en_US` and `en_GB` (‑ise) lists at size 60, the size of the standard Hunspell dictionaries |
| Used for | the checker's lexicon: a 1–2 character difference counts as a typo only if the result is not a dictionary word ([docs/02](docs/02-layer1-content.md), «Проверка ответов на ввод») |
| Where from | nixpkgs package `scowl` (version 2020.12.07, pinned by `flake.lock`), files `share/dict/wamerican.60` and `share/dict/wbritish.60`; upstream <http://wordlist.aspell.net/> |
| How it is changed | content-build lowercases the words, merges both lists, sorts them and writes one file per lexicon version (SHA-256 of the contents) next to the content bundle |
| License | SCOWL copyright: permissive, MIT-like; it collects notices of its sources (Moby, UKACD, WordNet, Ispell and others). Full text: [`licenses/SCOWL.txt`](licenses/SCOWL.txt), copied verbatim from the package |
| What it requires | the copyright and permission notices appear with every copy of the word list and in supporting documentation; the UKACD notice must be displayed prominently with the text of its license. Nothing restricts commercial use, and the lexicon does not make our content a derivative work |
| How we comply | `licenses/SCOWL.txt` lives in the repository; the server serves it next to every lexicon file, and the client shows it on its licenses screen (both from stage 1, when the lexicon is first shipped to devices) |
