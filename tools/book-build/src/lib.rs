//! The graded-reader compiler (`docs/16-vocabulary-library-notes-graph.md`,
//! §2). It reads a Nickel manifest and its text files, checks the license,
//! level and structure, and writes one immutable JSON asset per book. The
//! output depends only on the inputs: the same sources give the same bytes
//! and the same `version`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use refl_core::book::{Book, BookChapter, BookParagraph, BookSentence, BookToken};
use refl_core::bundle::Bundle;
use refl_core::content::Lexeme;
use refl_core::ids::{BookId, KcId, LexemeId};
use refl_core::subject::LevelKey;
use refl_core::value::{Lang, Localized};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Inputs of one build.
#[derive(Debug, Clone)]
pub struct Options {
    /// Directory with one subdirectory per book, each holding `book.ncl`.
    pub books: PathBuf,
    /// Output directory for `<book-id>.json` and `index.json`.
    pub out: PathBuf,
    /// The built content bundle, which supplies the lexeme bank.
    pub bundle: PathBuf,
    /// Repository `THIRD_PARTY.md` with a record for every book.
    pub third_party: PathBuf,
    /// `content/LICENSE` with the graded-reader terms.
    pub content_license: PathBuf,
    /// The `nickel` executable.
    pub nickel: PathBuf,
}

impl Options {
    pub fn new(books: impl Into<PathBuf>) -> Self {
        let books = books.into();
        let root = books
            .parent()
            .and_then(Path::parent)
            .unwrap_or(Path::new("."))
            .to_path_buf();
        Self {
            out: root.join("target/books"),
            bundle: root.join("target/bundle.json"),
            third_party: root.join("THIRD_PARTY.md"),
            content_license: root.join("content/LICENSE"),
            books,
            nickel: PathBuf::from("nickel"),
        }
    }
}

/// A book source as authored. Titles may be partial in the manifest; the
/// compiler requires uz, ru and en before publishing.
#[derive(Debug, Clone, Deserialize)]
pub struct BookSource {
    pub id: String,
    pub title: Localized,
    pub level: String,
    pub license: String,
    pub source: String,
    pub chapters: Vec<ChapterSource>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChapterSource {
    #[serde(default)]
    pub id: Option<String>,
    pub title: Localized,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub file: Option<String>,
}

/// One compiled book plus its byte digest.
#[derive(Debug, Clone)]
pub struct Compiled {
    pub book: Book,
    pub bytes: Vec<u8>,
    pub digest: String,
}

/// A build problem; all problems are collected before anything is written.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub book: Option<String>,
    pub message: String,
}

impl Diagnostic {
    fn book(id: &str, message: impl Into<String>) -> Self {
        Self {
            book: Some(id.to_owned()),
            message: message.into(),
        }
    }

    fn global(message: impl Into<String>) -> Self {
        Self {
            book: None,
            message: message.into(),
        }
    }
}

impl core::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match &self.book {
            Some(id) => write!(f, "{id}: {}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

/// Metadata of one published asset, the shape the active bundle will carry
/// in stage 4.3.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookMeta {
    pub id: BookId,
    pub version: String,
    pub bytes: u64,
    pub digest: String,
    pub level: LevelKey,
    pub title: Localized,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookIndex {
    pub books: Vec<BookMeta>,
}

/// Runs the whole build, writing nothing itself (the caller prints and
/// decides). Missing bundle or license files become diagnostics.
///
/// # Errors
/// The books directory cannot be read.
pub fn build(opts: &Options) -> std::io::Result<(Vec<Compiled>, Vec<Diagnostic>)> {
    let bundle = match std::fs::read_to_string(&opts.bundle) {
        Ok(json) => match serde_json::from_str::<Bundle>(&json) {
            Ok(bundle) => bundle,
            Err(error) => {
                return Ok((
                    Vec::new(),
                    vec![Diagnostic::global(format!(
                        "{}: invalid bundle: {error}",
                        opts.bundle.display()
                    ))],
                ));
            }
        },
        Err(error) => {
            return Ok((
                Vec::new(),
                vec![Diagnostic::global(format!(
                    "{}: {error}; run `just content` first",
                    opts.bundle.display()
                ))],
            ));
        }
    };
    let third_party = std::fs::read_to_string(&opts.third_party).unwrap_or_default();
    let content_license = std::fs::read_to_string(&opts.content_license).unwrap_or_default();
    let index = LexemeIndex::of(&bundle);
    let mut compiled = Vec::new();
    let mut diagnostics = Vec::new();
    let mut seen = BTreeSet::new();
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&opts.books)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    for dir in dirs {
        let manifest = dir.join("book.ncl");
        if !manifest.is_file() {
            continue;
        }
        let json = match nickel_export(&opts.nickel, &manifest) {
            Ok(json) => json,
            Err(message) => {
                diagnostics.push(Diagnostic::global(format!(
                    "{}: {message}",
                    manifest.display()
                )));
                continue;
            }
        };
        let source: BookSource = match serde_json::from_str(&json) {
            Ok(source) => source,
            Err(error) => {
                diagnostics.push(Diagnostic::global(format!(
                    "{}: {error}",
                    manifest.display()
                )));
                continue;
            }
        };
        if !seen.insert(source.id.clone()) {
            diagnostics.push(Diagnostic::book(
                &source.id,
                "book id is defined by more than one directory",
            ));
            continue;
        }
        if let Err(message) = check_license(
            &source.id,
            &source.license,
            &source.source,
            &third_party,
            &content_license,
        ) {
            diagnostics.push(Diagnostic::book(&source.id, message));
            continue;
        }
        match compile_book(&source, &dir, &index) {
            Ok(book) => {
                let bytes = book_bytes(&book);
                let digest = hex(&Sha256::digest(&bytes));
                compiled.push(Compiled {
                    book,
                    bytes,
                    digest,
                });
            }
            Err(messages) => {
                for message in messages {
                    diagnostics.push(Diagnostic::book(&source.id, message));
                }
            }
        }
    }
    Ok((compiled, diagnostics))
}

/// Writes the compiled books and `index.json`.
///
/// # Errors
/// The output directory or a file cannot be written.
///
/// # Panics
/// Never: the index always serializes.
pub fn write(opts: &Options, compiled: &[Compiled]) -> std::io::Result<()> {
    std::fs::create_dir_all(&opts.out)?;
    for item in compiled {
        std::fs::write(opts.out.join(format!("{}.json", item.book.id)), &item.bytes)?;
    }
    let index = BookIndex {
        books: compiled
            .iter()
            .map(|item| BookMeta {
                id: item.book.id.clone(),
                version: item.book.version.clone(),
                bytes: u64::try_from(item.bytes.len()).unwrap_or(u64::MAX),
                digest: item.digest.clone(),
                level: item.book.level.clone(),
                title: item.book.title.clone(),
            })
            .collect(),
    };
    let mut json = serde_json::to_vec(&index).expect("book index serializes");
    json.push(b'\n');
    std::fs::write(opts.out.join("index.json"), json)
}

/// The canonical asset bytes: compact JSON plus a trailing newline.
///
/// # Panics
/// Never: book types always serialize.
pub fn book_bytes(book: &Book) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(book).expect("book serializes");
    bytes.push(b'\n');
    bytes
}

/// SHA-256 that goes into `version` and into every book-mark event. It is
/// computed over the book with an empty `version` field, so it cannot depend
/// on itself.
///
/// # Panics
/// Never: book types always serialize.
pub fn book_version(book: &Book) -> String {
    let mut copy = book.clone();
    copy.version = String::new();
    let json = serde_json::to_vec(&copy).expect("book serializes");
    hex(&Sha256::digest(json))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

fn nickel_export(nickel: &Path, path: &Path) -> Result<String, String> {
    let out = Command::new(nickel)
        .arg("export")
        .arg("--format")
        .arg("json")
        .arg(path)
        .output()
        .map_err(|error| format!("cannot run {}: {error}", nickel.display()))?;
    if out.status.success() {
        String::from_utf8(out.stdout)
            .map_err(|error| format!("nickel output is not UTF-8: {error}"))
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim_end().to_owned())
    }
}

/// Licenses that may be mixed into the closed content set. Copyleft
/// licenses are deliberately absent: the spec forbids mixing them with
/// closed content without relicensing the whole derivative.
fn license_allowed(license: &str) -> bool {
    matches!(
        license.to_ascii_lowercase().as_str(),
        "original" | "public-domain" | "public domain" | "cc0" | "cc0-1.0" | "cc-by" | "cc-by-4.0"
    )
}

/// A book is publishable only when its license is known, its source is
/// named, and both license files carry its record.
fn check_license(
    id: &str,
    license: &str,
    source: &str,
    third_party: &str,
    content_license: &str,
) -> Result<(), String> {
    if license.trim().is_empty() {
        return Err("license is required".into());
    }
    if !license_allowed(license) {
        return Err(format!(
            "license `{license}` is unknown or copyleft; use original, public-domain, cc0 or cc-by"
        ));
    }
    if source.trim().is_empty() {
        return Err("source is required".into());
    }
    if !third_party.contains(id) {
        return Err(format!(
            "THIRD_PARTY.md has no record for `{id}`; add what the book is, where it comes from and its license"
        ));
    }
    if !content_license.contains(id) {
        return Err(format!(
            "content/LICENSE does not name `{id}`; add the graded-reader terms"
        ));
    }
    Ok(())
}

fn level_allowed(level: &str) -> bool {
    matches!(level, "A1" | "A2" | "B1" | "B2" | "C1")
}

fn require_localized(value: &Localized, what: &str, out: &mut Vec<String>) {
    for (lang, key) in [(Lang::En, "en"), (Lang::Ru, "ru"), (Lang::Uz, "uz")] {
        if value.get(lang).is_none_or(str::is_empty) {
            out.push(format!("{what}: title in {key} is missing"));
        }
    }
}

/// The identifier stem inside deterministic ids: `book.first-day` becomes
/// `first-day`.
fn slug(id: &str) -> &str {
    id.strip_prefix("book.").unwrap_or(id)
}

/// Paragraphs are separated by blank lines; newlines inside a paragraph are
/// soft wraps, not sentence breaks.
pub fn paragraph_texts(text: &str) -> Vec<String> {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = Vec::new();
    let mut lines: Vec<&str> = Vec::new();
    let flush = |lines: &mut Vec<&str>, out: &mut Vec<String>| {
        if lines.is_empty() {
            return;
        }
        let paragraph = lines
            .iter()
            .flat_map(|line| line.split_whitespace())
            .collect::<Vec<_>>()
            .join(" ");
        lines.clear();
        if !paragraph.is_empty() {
            out.push(paragraph);
        }
    };
    for line in normalized.lines() {
        if line.trim().is_empty() {
            flush(&mut lines, &mut out);
        } else {
            lines.push(line);
        }
    }
    flush(&mut lines, &mut out);
    out
}

/// Sentences end at `.`, `!` or `?`, keeping closing quotes and repeated
/// terminators. Whitespace after the terminator is not part of the sentence.
pub fn sentence_texts(paragraph: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = paragraph.char_indices().peekable();
    let mut start = 0;
    while let Some((i, c)) = chars.next() {
        if !matches!(c, '.' | '!' | '?') {
            continue;
        }
        let mut end = i + c.len_utf8();
        while let Some(&(j, next)) = chars.peek() {
            if matches!(next, '"' | '\'' | '”' | '’' | '!' | '?') {
                end = j + next.len_utf8();
                chars.next();
            } else {
                break;
            }
        }
        if chars.peek().is_some_and(|(_, next)| next.is_whitespace()) {
            let sentence = paragraph[start..end].trim();
            if !sentence.is_empty() {
                out.push(sentence.to_owned());
            }
            while chars.peek().is_some_and(|(_, next)| next.is_whitespace()) {
                chars.next();
            }
            start = chars.peek().map_or(paragraph.len(), |(j, _)| *j);
        }
    }
    let rest = paragraph[start..].trim();
    if !rest.is_empty() {
        out.push(rest.to_owned());
    }
    out
}

struct LexemeIndex {
    /// Form (lemma or inflection, lowercased) → candidate lexeme indices.
    by_form: BTreeMap<String, BTreeSet<usize>>,
    lexemes: Vec<Lexeme>,
}

impl LexemeIndex {
    fn of(bundle: &Bundle) -> Self {
        let mut lexemes = bundle.content.lexemes.clone();
        lexemes.sort_by(|left, right| left.id.cmp(&right.id));
        let mut by_form: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
        for (index, lexeme) in lexemes.iter().enumerate() {
            let mut forms = BTreeSet::new();
            forms.insert(lexeme.lemma.to_ascii_lowercase());
            forms.extend(lexeme.forms.iter().map(|form| form.to_ascii_lowercase()));
            for form in forms {
                by_form.entry(form).or_default().insert(index);
            }
        }
        Self { by_form, lexemes }
    }

    fn link(&self, word: &str) -> (Option<LexemeId>, Vec<LexemeId>, Vec<KcId>, String) {
        let candidates = self.by_form.get(word).cloned().unwrap_or_default();
        let mut senses = BTreeSet::new();
        let mut kcs = BTreeSet::new();
        let mut lemma = None;
        for index in &candidates {
            let lexeme = &self.lexemes[*index];
            if candidates.len() == 1 {
                lemma = Some(lexeme.lemma.clone());
            }
            senses.extend(lexeme.senses.iter().map(|sense| sense.id.clone()));
            kcs.extend(lexeme.kc_ids.iter().cloned());
        }
        let senses: Vec<LexemeId> = senses.into_iter().collect();
        let (lexeme_id, sense_ids) = if senses.len() == 1 {
            (Some(senses[0].clone()), Vec::new())
        } else {
            (None, senses)
        };
        (
            lexeme_id,
            sense_ids,
            kcs.into_iter().collect(),
            lemma.unwrap_or_else(|| word.to_owned()),
        )
    }
}

fn safe_file(root: &Path, file: &str) -> Result<PathBuf, String> {
    let path = Path::new(file);
    if path.is_absolute()
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
    {
        return Err(format!(
            "chapter file `{file}` must stay inside the book directory"
        ));
    }
    Ok(root.join(path))
}

/// Compiles one parsed manifest with the bundle's lexeme bank. `root`
/// resolves chapter files.
///
/// # Errors
/// Every problem with the manifest, chapter files and token linking,
/// collected instead of stopping at the first.
pub fn compile_source(
    source: &BookSource,
    root: &Path,
    bundle: &Bundle,
) -> Result<Book, Vec<String>> {
    compile_book(source, root, &LexemeIndex::of(bundle))
}

fn compile_book(
    source: &BookSource,
    root: &Path,
    index: &LexemeIndex,
) -> Result<Book, Vec<String>> {
    let mut errors = Vec::new();
    if !source.id.starts_with("book.") || BookId::new(&source.id).is_none() {
        errors.push(format!(
            "id `{}` must be a slug starting with `book.`",
            source.id
        ));
    }
    if !level_allowed(&source.level) {
        errors.push(format!(
            "level `{}` must be A1, A2, B1, B2 or C1",
            source.level
        ));
    }
    require_localized(&source.title, "book", &mut errors);
    if source.chapters.is_empty() {
        errors.push("a book needs at least one chapter".into());
    }
    let book_slug = slug(&source.id).to_owned();
    let mut chapters = Vec::new();
    for (c, chapter) in source.chapters.iter().enumerate() {
        let number = c + 1;
        let generated = format!("chapter.{book_slug}.{number}");
        if chapter.id.as_deref().is_some_and(|id| id != generated) {
            errors.push(format!(
                "chapter id `{}` must be `{generated}`",
                chapter.id.as_deref().unwrap_or_default()
            ));
        }
        require_localized(&chapter.title, &generated, &mut errors);
        let text = match (&chapter.text, &chapter.file) {
            (Some(text), None) if !text.trim().is_empty() => text.clone(),
            (None, Some(file)) => {
                let path = match safe_file(root, file) {
                    Ok(path) => path,
                    Err(message) => {
                        errors.push(message);
                        continue;
                    }
                };
                match std::fs::read_to_string(&path) {
                    Ok(text) => text,
                    Err(error) => {
                        errors.push(format!("chapter file `{file}`: {error}"));
                        continue;
                    }
                }
            }
            (Some(_), Some(_)) => {
                errors.push(format!("{generated}: give either text or file, not both"));
                continue;
            }
            _ => {
                errors.push(format!("{generated}: chapter text is empty"));
                continue;
            }
        };
        chapters.push(build_chapter(
            &book_slug,
            number,
            chapter.title.clone(),
            &text,
            index,
            &mut errors,
        ));
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut book = Book {
        id: BookId::new(source.id.clone()).expect("validated slug"),
        version: String::new(),
        title: source.title.clone(),
        level: LevelKey::new(source.level.clone()).expect("validated level"),
        license: source.license.clone(),
        source: source.source.clone(),
        chapters,
    };
    book.version = book_version(&book);
    Ok(book)
}

fn build_chapter(
    book: &str,
    number: usize,
    title: Localized,
    text: &str,
    index: &LexemeIndex,
    errors: &mut Vec<String>,
) -> BookChapter {
    let chapter_id = format!("chapter.{book}.{number}");
    let paragraphs = paragraph_texts(text);
    if paragraphs.is_empty() {
        errors.push(format!("{chapter_id}: chapter text is empty"));
    }
    let mut built = Vec::with_capacity(paragraphs.len());
    for (p, paragraph) in paragraphs.iter().enumerate() {
        let paragraph_number = p + 1;
        let paragraph_id = format!("para.{book}.{number}.{paragraph_number}");
        let sentences = sentence_texts(paragraph);
        if sentences.is_empty() {
            errors.push(format!("{paragraph_id}: paragraph has no sentences"));
        }
        let mut built_sentences = Vec::with_capacity(sentences.len());
        for (s, sentence) in sentences.iter().enumerate() {
            let sentence_number = s + 1;
            let sentence_id =
                format!("sentence.{book}.{number}.{paragraph_number}.{sentence_number}");
            let words: Vec<&str> = content_build::word_spans(sentence).collect();
            if words.is_empty() {
                errors.push(format!("{sentence_id}: sentence has no word tokens"));
                continue;
            }
            let mut tokens = Vec::with_capacity(words.len());
            for (t, word) in words.iter().enumerate() {
                let token_number = t + 1;
                let lookup = word.to_ascii_lowercase();
                let (lexeme_id, sense_ids, kc_ids, lemma) = index.link(&lookup);
                tokens.push(BookToken {
                    id: format!(
                        "token.{book}.{number}.{paragraph_number}.{sentence_number}.{token_number}"
                    ),
                    text: (*word).to_owned(),
                    lemma,
                    lexeme_id,
                    sense_ids,
                    kc_ids,
                });
            }
            built_sentences.push(BookSentence {
                id: sentence_id,
                text: sentence.clone(),
                tokens,
            });
        }
        built.push(BookParagraph {
            id: paragraph_id,
            sentences: built_sentences,
        });
    }
    BookChapter {
        id: chapter_id,
        title,
        paragraphs: built,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use refl_core::content::{LangReviews, LexemePos, LexemeSense};
    use std::collections::BTreeMap;

    fn localized(en: &str) -> Localized {
        Localized {
            uz: Some(en.into()),
            ru: Some(en.into()),
            en: Some(en.into()),
        }
    }

    fn lexeme(id: &str, sense: &str, form: &str) -> Lexeme {
        Lexeme {
            id: LexemeId::new(id).unwrap(),
            lemma: form.into(),
            pos: LexemePos::Noun,
            frequency_rank: 10,
            level: LevelKey::new("A1").unwrap(),
            forms: vec![format!("{form}s")],
            senses: vec![LexemeSense {
                id: LexemeId::new(sense).unwrap(),
                definition_en: "a test word".into(),
                gloss_uz: String::new(),
                gloss_ru: String::new(),
                examples: Vec::new(),
                collocations: Vec::new(),
                ipa_uk: String::new(),
                ipa_us: String::new(),
                audio_key: None,
                review: LangReviews::default(),
            }],
            kc_ids: vec![KcId::new("en.gram.article.a-an-sound").unwrap()],
            source: "test".into(),
            license: "original".into(),
            review: LangReviews::default(),
        }
    }

    fn index() -> LexemeIndex {
        let lexemes = vec![lexeme("en.lex.cat", "en.lex.cat.animal", "cat")];
        let by_form = BTreeMap::from([
            ("cat".to_owned(), BTreeSet::from([0usize])),
            ("cats".to_owned(), BTreeSet::from([0usize])),
        ]);
        LexemeIndex { by_form, lexemes }
    }

    #[test]
    fn paragraphs_and_sentences_are_deterministic() {
        let paragraphs = paragraph_texts("One line\r\ncontinues here.\n\nSecond paragraph!\n");
        assert_eq!(
            paragraphs,
            vec!["One line continues here.", "Second paragraph!"]
        );
        assert_eq!(
            sentence_texts("He said, \"Come in.\" Then he left. Ok?"),
            vec!["He said, \"Come in.\"", "Then he left.", "Ok?"]
        );
    }

    #[test]
    fn allowed_and_rejected_licenses() {
        assert!(license_allowed("original"));
        assert!(license_allowed("Public-Domain"));
        assert!(license_allowed("cc-by"));
        assert!(!license_allowed("cc-by-sa"));
        assert!(!license_allowed("unknown"));
    }

    #[test]
    fn token_link_uses_sense_ids_and_inflections() {
        let (id, senses, kcs, lemma) = index().link("cats");
        assert_eq!(id, LexemeId::new("en.lex.cat.animal"));
        assert_eq!(senses, Vec::new());
        assert_eq!(kcs, vec![KcId::new("en.gram.article.a-an-sound").unwrap()]);
        assert_eq!(lemma, "cat");
        let (id, senses, _, lemma) = index().link("dog");
        assert!(id.is_none());
        assert_eq!(senses, Vec::new());
        assert_eq!(lemma, "dog");
    }

    #[test]
    fn book_version_is_stable_and_matches_bytes() {
        let source = BookSource {
            id: "book.test".into(),
            title: localized("Test"),
            level: "A1".into(),
            license: "original".into(),
            source: "refl test".into(),
            chapters: vec![ChapterSource {
                id: Some("chapter.test.1".into()),
                title: localized("One"),
                text: Some("A cat sits. A cat naps.".into()),
                file: None,
            }],
        };
        let book = compile_book(&source, Path::new("."), &index()).unwrap();
        assert_eq!(book.chapters[0].paragraphs[0].sentences.len(), 2);
        let sentence = &book.chapters[0].paragraphs[0].sentences[0];
        assert_eq!(sentence.tokens[0].text, "A");
        assert_eq!(sentence.tokens[1].text, "cat");
        assert_eq!(sentence.tokens[1].lemma, "cat");
        assert_eq!(
            book.chapters[0].paragraphs[0].sentences[0].tokens[1].id,
            "token.test.1.1.1.2"
        );
        let first = book_bytes(&book);
        let again = compile_book(&source, Path::new("."), &index()).unwrap();
        assert_eq!(book_bytes(&again), first);
        assert_eq!(again.version, book.version);
        assert_eq!(
            book.version,
            hex(&Sha256::digest({
                let mut copy = book.clone();
                copy.version = String::new();
                serde_json::to_vec(&copy).unwrap()
            }))
        );
    }
}
