//! Stage 1 book importer (`docs/16-vocabulary-library-notes-graph.md`, §2).
//!
//! The importer is deliberately a *source* tool, not a reader: it turns an
//! EPUB, a text-layer PDF, TXT or Markdown into the canonical
//! `source.md` (YAML frontmatter + `##` chapters) that `book-build` consumes.
//! Structural repair is best effort and every intervention is reported as a
//! warning; an empty or corrupt chapter is an error, never output.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

const MIN_PAGE_CHARS: usize = 40;
const SYNTHETIC_CHAPTER_CHARS: usize = 3_500;

/// Inputs of one import.
#[derive(Debug, Clone)]
pub struct Options {
    pub input: PathBuf,
    /// Directory for `content/books/<slug>/source.md`.
    pub out: PathBuf,
    /// `book.<slug>`; derived from the file name when absent.
    pub id: Option<String>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub level: String,
    pub license: String,
    pub source: Option<String>,
    pub force: bool,
}

impl Options {
    pub fn new(input: impl Into<PathBuf>) -> Self {
        Self {
            input: input.into(),
            out: PathBuf::from("content/books"),
            id: None,
            title: None,
            author: None,
            level: "A1".into(),
            license: String::new(),
            source: None,
            force: false,
        }
    }
}

/// What was written.
#[derive(Debug, Clone)]
pub struct Imported {
    pub path: PathBuf,
    pub book_id: String,
    pub chapters: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Epub,
    Pdf,
    Text,
    Markdown,
}

fn format_of(path: &Path) -> Result<Format, String> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("epub") => Ok(Format::Epub),
        Some("pdf") => Ok(Format::Pdf),
        Some("txt") => Ok(Format::Text),
        Some("md" | "markdown") => Ok(Format::Markdown),
        _ => Err(format!(
            "{}: expected .epub, .pdf, .txt or .md",
            path.display()
        )),
    }
}

#[derive(Debug, Clone)]
struct Block {
    heading: bool,
    text: String,
}

#[derive(Debug, Default)]
struct Meta {
    title: Option<String>,
    author: Option<String>,
}

#[derive(Debug)]
struct Extracted {
    meta: Meta,
    blocks: Vec<Block>,
    warnings: Vec<String>,
}

/// Imports one file and writes its canonical `source.md`.
///
/// # Errors
/// The input is unreadable, the format is unsupported, a scanned PDF cannot
/// be OCR'd, the text is empty, or the output exists without `force`.
pub fn import(opts: &Options) -> Result<Imported, String> {
    let format = format_of(&opts.input)?;
    let extracted = match format {
        Format::Epub => extract_epub(&opts.input)?,
        Format::Pdf => extract_pdf(&opts.input)?,
        Format::Text | Format::Markdown => extract_text(&opts.input, format)?,
    };
    let stem = opts
        .input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("book");
    let book_id = opts
        .id
        .clone()
        .unwrap_or_else(|| format!("book.{}", slugify(stem)));
    if !book_id.starts_with("book.") || book_id.len() <= "book.".len() {
        return Err(format!(
            "id `{book_id}` must be a slug starting with `book.`"
        ));
    }
    let title = opts
        .title
        .clone()
        .or(extracted.meta.title)
        .unwrap_or_else(|| title_from_stem(stem));
    let author = opts
        .author
        .clone()
        .or(extracted.meta.author)
        .unwrap_or_default();
    let source = opts
        .source
        .clone()
        .unwrap_or_else(|| format!("Imported from {}", display_name(&opts.input)));
    if opts.license.trim().is_empty() {
        return Err("--license is required (original, public-domain, cc0 or cc-by)".into());
    }
    let mut warnings = extracted.warnings;
    let chapters = assemble(&title, extracted.blocks, &mut warnings)?;
    let markdown = render(
        &book_id,
        &title,
        &author,
        &opts.level,
        &opts.license,
        &source,
        &chapters,
    );
    let folder = book_id.trim_start_matches("book.");
    let dir = opts.out.join(folder);
    let path = dir.join("source.md");
    if path.exists() && !opts.force {
        return Err(format!(
            "{} already exists; pass --force to replace it",
            path.display()
        ));
    }
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("cannot create {}: {error}", dir.display()))?;
    std::fs::write(&path, markdown)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(Imported {
        path,
        book_id,
        chapters: chapters.len(),
        warnings,
    })
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file")
        .to_owned()
}

fn title_from_stem(stem: &str) -> String {
    stem.replace(['_', '-'], " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + chars.as_str()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn slugify(text: &str) -> String {
    let mut slug = String::new();
    let mut separator = false;
    for ch in text.chars() {
        let ch = ch.to_ascii_lowercase();
        if ch.is_ascii_alphanumeric() {
            if separator && !slug.is_empty() {
                slug.push('-');
            }
            separator = false;
            slug.push(ch);
        } else {
            separator = true;
        }
    }
    if slug.is_empty() {
        "untitled".into()
    } else {
        slug
    }
}

// ---------------------------------------------------------------- Markdown

struct Chapter {
    title: String,
    paragraphs: Vec<String>,
}

/// Assembles blocks into chapters, repairing or synthesizing headings.
#[allow(clippy::too_many_lines)]
fn assemble(
    book_title: &str,
    blocks: Vec<Block>,
    warnings: &mut Vec<String>,
) -> Result<Vec<Chapter>, String> {
    let mut blocks = blocks
        .into_iter()
        .filter_map(|block| {
            let text = normalize_text(&block.text);
            (!text.trim().is_empty()).then_some(Block {
                heading: block.heading,
                text: text.trim().to_owned(),
            })
        })
        .collect::<Vec<_>>();
    // A heading-looking short line from a text extractor becomes a heading.
    for block in &mut blocks {
        if !block.heading && heading_like(&block.text) {
            block.heading = true;
        }
    }
    let headings = blocks.iter().any(|block| block.heading);
    if !headings {
        let mut chapters = Vec::new();
        let mut current: Vec<String> = Vec::new();
        let mut chars = 0usize;
        for block in blocks {
            if !current.is_empty() && chars + block.text.len() > SYNTHETIC_CHAPTER_CHARS {
                chapters.push(Chapter {
                    title: format!("Part {}", chapters.len() + 1),
                    paragraphs: std::mem::take(&mut current),
                });
                chars = 0;
            }
            chars += block.text.len();
            current.push(block.text);
        }
        if !current.is_empty() {
            chapters.push(Chapter {
                title: if chapters.is_empty() {
                    book_title.to_owned()
                } else {
                    format!("Part {}", chapters.len() + 1)
                },
                paragraphs: current,
            });
        }
        if chapters.is_empty() {
            return Err("the document has no text".into());
        }
        warnings.push(format!(
            "no chapter headings found; synthesized {} chapter(s)",
            chapters.len()
        ));
        return Ok(chapters);
    }
    let mut chapters: Vec<Chapter> = Vec::new();
    let mut body: Vec<String> = Vec::new();
    let mut current_title: Option<String> = None;
    for block in blocks {
        if block.heading {
            if let Some(title) = current_title.take() {
                if body.is_empty() {
                    warnings.push(format!("chapter `{title}` had no text and was dropped"));
                } else {
                    chapters.push(Chapter {
                        title,
                        paragraphs: take_paragraphs(&mut body),
                    });
                }
            } else if !body.is_empty() {
                warnings.push("text before the first heading became `Beginning`".into());
                chapters.push(Chapter {
                    title: "Beginning".into(),
                    paragraphs: take_paragraphs(&mut body),
                });
            }
            current_title = Some(block.text);
        } else {
            body.push(block.text);
        }
    }
    if let Some(title) = current_title {
        if body.is_empty() {
            warnings.push(format!("chapter `{title}` had no text and was dropped"));
        } else {
            chapters.push(Chapter {
                title,
                paragraphs: take_paragraphs(&mut body),
            });
        }
    } else if !body.is_empty() {
        warnings.push("text before the first heading became `Beginning`".into());
        chapters.push(Chapter {
            title: "Beginning".into(),
            paragraphs: take_paragraphs(&mut body),
        });
    }
    if chapters.is_empty() {
        return Err("the document has headings but no chapter text".into());
    }
    // Duplicate chapter titles become stable and distinct.
    let mut seen = BTreeSet::new();
    for chapter in &mut chapters {
        let base = chapter.title.clone();
        let mut n = 2;
        while !seen.insert(chapter.title.clone()) {
            chapter.title = format!("{base} ({n})");
            n += 1;
        }
    }
    Ok(chapters)
}

fn take_paragraphs(buf: &mut Vec<String>) -> Vec<String> {
    std::mem::take(buf)
}

fn heading_like(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.len() > 80 {
        return false;
    }
    if trimmed.ends_with(['.', ',', ';', ':']) {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    let numbered = lower
        .split_once(' ')
        .is_some_and(|(first, _)| first.trim_end_matches(['.', ')']).parse::<u32>().is_ok());
    let named = ["chapter", "part", "book", "section", "prologue", "epilogue"]
        .iter()
        .any(|word| lower.starts_with(word));
    let words = trimmed.split_whitespace().count();
    let upper = words > 0
        && words <= 8
        && trimmed
            .chars()
            .filter(|ch| ch.is_alphabetic())
            .all(|ch| !ch.is_lowercase());
    let title_case = words > 0
        && words <= 6
        && trimmed
            .split_whitespace()
            .filter(|word| word.chars().next().is_some_and(char::is_uppercase))
            .count()
            >= words.saturating_sub(1);
    numbered || named || upper || title_case
}

fn render(
    id: &str,
    title: &str,
    author: &str,
    level: &str,
    license: &str,
    source: &str,
    chapters: &[Chapter],
) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    let _ = writeln!(out, "id: {}", yaml(id));
    let _ = writeln!(out, "title: {}", yaml(title));
    let _ = writeln!(out, "author: {}", yaml(author));
    let _ = writeln!(out, "level: {}", yaml(level));
    let _ = writeln!(out, "license: {}", yaml(license));
    let _ = writeln!(out, "source: {}", yaml(source));
    out.push_str("---\n\n");
    for chapter in chapters {
        let _ = writeln!(out, "## {}", chapter.title.trim());
        out.push('\n');
        for paragraph in &chapter.paragraphs {
            out.push_str(paragraph.trim());
            out.push_str("\n\n");
        }
    }
    out.trim_end().to_owned() + "\n"
}

fn yaml(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

// ---------------------------------------------------------------- text/MD

fn extract_text(path: &Path, format: Format) -> Result<Extracted, String> {
    let bytes =
        std::fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let text = String::from_utf8(bytes)
        .map_err(|error| format!("{} is not UTF-8: {error}", path.display()))?;
    let (text, healed) = heal_hyphenation(&text);
    let mut warnings = Vec::new();
    if healed > 0 {
        warnings.push(format!("healed {healed} end-of-line hyphenation split(s)"));
    }
    let blocks = match format {
        Format::Markdown => markdown_blocks(&text),
        _ => paragraph_blocks(&text),
    };
    Ok(Extracted {
        meta: Meta::default(),
        blocks,
        warnings,
    })
}

fn markdown_blocks(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut paragraph = Vec::new();
    let flush = |paragraph: &mut Vec<&str>, blocks: &mut Vec<Block>| {
        if !paragraph.is_empty() {
            blocks.push(Block {
                heading: false,
                text: paragraph.join(" "),
            });
            paragraph.clear();
        }
    };
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix('#') {
            flush(&mut paragraph, &mut blocks);
            let heading = heading.trim_start_matches('#').trim();
            if !heading.is_empty() {
                blocks.push(Block {
                    heading: true,
                    text: heading.to_owned(),
                });
            }
        } else if trimmed.is_empty() {
            flush(&mut paragraph, &mut blocks);
        } else {
            paragraph.push(trimmed);
        }
    }
    flush(&mut paragraph, &mut blocks);
    blocks
}

fn paragraph_blocks(text: &str) -> Vec<Block> {
    let normalized = normalize_text(text);
    let mut blocks = Vec::new();
    for paragraph in normalized.split("\n\n") {
        let paragraph = paragraph.trim();
        if paragraph.is_empty() {
            continue;
        }
        blocks.push(Block {
            heading: false,
            text: paragraph.to_owned(),
        });
    }
    blocks
}

fn heal_hyphenation(text: &str) -> (String, usize) {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = normalized.lines().collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut healed = 0usize;
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i];
        if line.trim_end().ends_with('-')
            && !line.trim_end().ends_with("--")
            && i + 1 < lines.len()
            && lines[i + 1]
                .trim_start()
                .chars()
                .next()
                .is_some_and(char::is_lowercase)
            && !lines[i + 1].trim().is_empty()
        {
            let joined = format!(
                "{}{}",
                line.trim_end().trim_end_matches('-'),
                lines[i + 1].trim_start()
            );
            out.push(joined);
            healed += 1;
            i += 2;
        } else {
            out.push(line.to_owned());
            i += 1;
        }
    }
    (out.join("\n"), healed)
}

// ---------------------------------------------------------------- PDF

/// One PDF pipeline; the stages belong together for the scan rules.
#[allow(clippy::too_many_lines)]
fn extract_pdf(path: &Path) -> Result<Extracted, String> {
    let bytes =
        std::fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut pages = pdf_extract::extract_text_from_mem_by_pages(&bytes)
        .map_err(|error| format!("cannot extract PDF text from {}: {error}", path.display()))?;
    if pages.is_empty() {
        return Err(format!(
            "{}: the PDF has no pages or no readable text",
            path.display()
        ));
    }
    let mut warnings = Vec::new();
    let removed = strip_running_lines(&mut pages);
    if removed > 0 {
        warnings.push(format!(
            "removed {removed} running header/footer or page-number line(s)"
        ));
    }
    let scanned: Vec<usize> = pages
        .iter()
        .enumerate()
        .filter(|(_, page)| non_whitespace(page) < MIN_PAGE_CHARS)
        .map(|(index, _)| index)
        .collect();
    if !scanned.is_empty() {
        let pages_ocr = ocr_pdf(path, &scanned)?;
        for (index, text) in pages_ocr {
            if let Some(page) = pages.get_mut(index) {
                *page = text;
            }
        }
        warnings.push(format!(
            "{} page(s) looked scanned; text came from tesseract OCR",
            scanned.len()
        ));
    }
    let mut text = String::new();
    for page in &pages {
        let (page, healed) = heal_hyphenation(page);
        if healed > 0 {
            warnings.push(format!("healed {healed} hyphenation split(s)"));
        }
        if non_whitespace(&page) < MIN_PAGE_CHARS {
            return Err(format!(
                "{}: a page still has no text after extraction; provide a clean text-layer PDF or EPUB",
                path.display()
            ));
        }
        text.push_str(&page);
        text.push_str("\n\n");
    }
    Ok(Extracted {
        meta: Meta::default(),
        blocks: paragraph_blocks(&text),
        warnings,
    })
}

fn non_whitespace(text: &str) -> usize {
    text.chars().filter(|ch| !ch.is_whitespace()).count()
}

fn strip_running_lines(pages: &mut [String]) -> usize {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for page in pages.iter() {
        let lines = page_lines(page);
        for line in lines.iter().take(2).chain(lines.iter().rev().take(2)) {
            if line.len() <= 80 {
                *counts.entry(line.clone()).or_default() += 1;
            }
        }
    }
    let page_count = pages.len();
    let threshold = (page_count / 2).max(2);
    let mut removed = 0usize;
    for page in pages.iter_mut() {
        let lines = page_lines(page);
        let kept: Vec<String> = lines
            .into_iter()
            .filter(|line| {
                let repeated = counts.get(line).copied().unwrap_or(0);
                let upper = line
                    .chars()
                    .filter(|ch| ch.is_alphabetic())
                    .all(|ch| !ch.is_lowercase());
                let header = repeated >= threshold
                    && (line.chars().any(|ch| ch.is_ascii_digit())
                        || upper
                        || (repeated * 10 >= page_count * 7
                            && line.len() < 40
                            && !line.ends_with(['.', '!', '?'])));
                let drop = page_number(line) || header;
                if drop {
                    removed += 1;
                }
                !drop
            })
            .collect();
        *page = kept.join("\n");
    }
    removed
}

fn page_lines(page: &str) -> Vec<String> {
    page.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn page_number(line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() || line.len() > 10 {
        return false;
    }
    if line.chars().all(|ch| ch.is_ascii_digit()) {
        return true;
    }
    let lower = line.to_ascii_lowercase();
    if lower
        .strip_prefix("page ")
        .is_some_and(|rest| rest.chars().all(|ch| ch.is_ascii_digit()))
        || lower
            .strip_prefix("p. ")
            .is_some_and(|rest| rest.chars().all(|ch| ch.is_ascii_digit()))
    {
        return true;
    }
    matches!(
        lower.as_str(),
        "i" | "ii"
            | "iii"
            | "iv"
            | "v"
            | "vi"
            | "vii"
            | "viii"
            | "ix"
            | "x"
            | "xi"
            | "xii"
            | "xiii"
            | "xiv"
            | "xv"
    )
}

fn command_exists(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn ocr_pdf(path: &Path, pages: &[usize]) -> Result<Vec<(usize, String)>, String> {
    if !command_exists("tesseract") {
        return Err(format!(
            "{}: {} page(s) look scanned. Install tesseract to OCR them, or provide a text-layer PDF/EPUB.",
            path.display(),
            pages.len()
        ));
    }
    if !command_exists("pdftoppm") {
        return Err(format!(
            "{}: {} page(s) look scanned. tesseract is installed, but pdftoppm (poppler) is needed to rasterize PDF pages; install poppler or provide a text-layer PDF/EPUB.",
            path.display(),
            pages.len()
        ));
    }
    let dir = std::env::temp_dir().join(format!(
        "refl-book-import-{}-{}",
        std::process::id(),
        pages.len()
    ));
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("cannot create temporary OCR directory: {error}"))?;
    let mut out = Vec::new();
    for (n, zero_index) in pages.iter().enumerate() {
        let page = zero_index + 1;
        let prefix = dir.join(format!("page-{n}"));
        let rendered = Command::new("pdftoppm")
            .args(["-f", &page.to_string(), "-l", &page.to_string()])
            .args(["-r", "200", "-png"])
            .arg(path)
            .arg(&prefix)
            .output()
            .map_err(|error| format!("cannot run pdftoppm: {error}"))?;
        if !rendered.status.success() {
            let _ = std::fs::remove_dir_all(&dir);
            return Err(format!(
                "pdftoppm failed on page {page}: {}",
                String::from_utf8_lossy(&rendered.stderr).trim()
            ));
        }
        let image = std::fs::read_dir(&dir)
            .map_err(|error| format!("cannot list OCR output: {error}"))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|candidate| {
                candidate
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .is_some_and(|stem| stem.starts_with(&format!("page-{n}-")))
            })
            .ok_or_else(|| format!("pdftoppm did not produce page {page}"))?;
        let text = Command::new("tesseract")
            .arg(&image)
            .arg("stdout")
            .args(["-l", "eng", "--psm", "3"])
            .output()
            .map_err(|error| format!("cannot run tesseract: {error}"))?;
        if !text.status.success() {
            let _ = std::fs::remove_dir_all(&dir);
            return Err(format!(
                "tesseract failed on page {page}: {}",
                String::from_utf8_lossy(&text.stderr).trim()
            ));
        }
        let text = String::from_utf8_lossy(&text.stdout).trim().to_owned();
        if non_whitespace(&text) < MIN_PAGE_CHARS {
            let _ = std::fs::remove_dir_all(&dir);
            return Err(format!(
                "page {page} still has no readable text after OCR; check the scan quality or provide a text-layer PDF/EPUB"
            ));
        }
        out.push((*zero_index, text));
    }
    let _ = std::fs::remove_dir_all(&dir);
    Ok(out)
}

// ---------------------------------------------------------------- EPUB

fn extract_epub(path: &Path) -> Result<Extracted, String> {
    let file = std::fs::File::open(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| format!("{} is not a readable EPUB: {error}", path.display()))?;
    let container = read_entry(&mut archive, "META-INF/container.xml")
        .ok_or_else(|| format!("{}: EPUB has no META-INF/container.xml", path.display()))?;
    let opf_path = attr_all(&container, "rootfile", "full-path")
        .or_else(|| full_path(&container))
        .ok_or_else(|| format!("{}: EPUB container has no rootfile", path.display()))?;
    let opf = read_entry(&mut archive, &opf_path)
        .ok_or_else(|| format!("{}: EPUB is missing {opf_path}", path.display()))?;
    let mut meta = Meta {
        title: element_text(&opf, "title"),
        author: element_text(&opf, "creator"),
    };
    if meta.title.is_none() {
        meta.title = element_text(&opf, "dc:title");
    }
    if meta.author.is_none() {
        meta.author = element_text(&opf, "dc:creator");
    }
    let opf_dir = Path::new(&opf_path)
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .to_path_buf();
    let mut manifest: BTreeMap<String, (String, String)> = BTreeMap::new();
    each_tag(&opf, |name, attrs| {
        if local_name(name) == "item" {
            let id = attrs.get("id").cloned().unwrap_or_default();
            let href = attrs.get("href").cloned().unwrap_or_default();
            let media = attrs.get("media-type").cloned().unwrap_or_default();
            if !id.is_empty() && !href.is_empty() {
                manifest.insert(id, (href, media));
            }
        }
    });
    let mut spine = Vec::new();
    each_tag(&opf, |name, attrs| {
        if local_name(name) == "itemref"
            && let Some(idref) = attrs.get("idref")
        {
            spine.push(idref.clone());
        }
    });
    if spine.is_empty() {
        spine = manifest
            .iter()
            .filter(|(_, (_, media))| media.contains("xhtml") || media.contains("html"))
            .map(|(id, _)| id.clone())
            .collect();
        spine.sort();
    }
    let mut blocks = Vec::new();
    let mut warnings = Vec::new();
    let mut files = 0usize;
    for idref in spine {
        let Some((href, _)) = manifest.get(&idref) else {
            continue;
        };
        let entry = resolve(&opf_dir, href);
        let Some(html) = read_entry(&mut archive, &entry) else {
            warnings.push(format!("spine item `{idref}` is missing from the EPUB"));
            continue;
        };
        let extracted = html_blocks(&html);
        if extracted.is_empty() {
            continue;
        }
        files += 1;
        blocks.extend(extracted);
    }
    if files == 0 || blocks.is_empty() {
        return Err(format!(
            "{}: EPUB has no readable XHTML text in its spine",
            path.display()
        ));
    }
    Ok(Extracted {
        meta,
        blocks,
        warnings,
    })
}

fn read_entry(archive: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<String> {
    let mut entry = archive.by_name(name).ok()?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

fn resolve(base: &Path, href: &str) -> String {
    let path = base.join(href);
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out.to_string_lossy().replace('\\', "/")
}

fn full_path(container: &str) -> Option<String> {
    let mut found = None;
    each_tag(container, |name, attrs| {
        if local_name(name) == "rootfile" {
            found = attrs.get("full-path").cloned();
        }
    });
    found
}

fn attr_all(xml: &str, tag: &str, attr: &str) -> Option<String> {
    let mut found = None;
    each_tag(xml, |name, attrs| {
        if local_name(name) == tag
            && let Some(value) = attrs.get(attr)
        {
            found = Some(value.clone());
        }
    });
    found
}

fn local_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

/// Calls `f` for every `<tag ...>` in `xml` with its attributes.
fn each_tag(xml: &str, mut f: impl FnMut(&str, &BTreeMap<String, String>)) {
    let mut rest = xml;
    while let Some(start) = rest.find('<') {
        rest = &rest[start + 1..];
        let Some(end) = rest.find('>') else {
            break;
        };
        let tag = &rest[..end];
        if !tag.starts_with('!') && !tag.starts_with('?') {
            let name = tag
                .trim_start_matches('/')
                .split_whitespace()
                .next()
                .unwrap_or("");
            let attrs = parse_attrs(tag);
            f(name, &attrs);
        }
        rest = &rest[end + 1..];
    }
}

fn parse_attrs(tag: &str) -> BTreeMap<String, String> {
    let mut attrs = BTreeMap::new();
    let bytes = tag.as_bytes();
    let mut i = tag.find(char::is_whitespace).unwrap_or(tag.len());
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'=' {
            i += 1;
        }
        let name = tag[start..i].trim();
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            i += 1;
            continue;
        }
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let quote = if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
            let quote = bytes[i];
            i += 1;
            quote
        } else {
            b' '
        };
        let value_start = i;
        if quote == b' ' {
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
        } else {
            while i < bytes.len() && bytes[i] != quote {
                i += 1;
            }
        }
        let value = &tag[value_start..i];
        if quote != b' ' && i < bytes.len() {
            i += 1;
        }
        if !name.is_empty() {
            attrs.insert(name.to_owned(), decode_entities(value));
        }
    }
    attrs
}

fn element_text(xml: &str, name: &str) -> Option<String> {
    let open = ["<", name].concat();
    let close = format!("</{name}>");
    let start = xml.find(&open)?;
    let after = xml[start..].find('>')? + start + 1;
    let end = xml[after..].find(&close)? + after;
    let text = decode_entities(&xml[after..end]);
    let text = normalize_text(&text);
    (!text.trim().is_empty()).then(|| text.trim().to_owned())
}

fn html_blocks(html: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut text = String::new();
    let mut heading = false;
    let mut skip: Option<String> = None;
    let mut rest = html;
    while let Some(start) = rest.find('<') {
        if skip.is_none() {
            let chunk = decode_entities(&rest[..start]);
            text.push_str(&chunk);
        }
        rest = &rest[start + 1..];
        let Some(end) = rest.find('>') else {
            break;
        };
        let tag = &rest[..end];
        let closing = tag.starts_with('/');
        let name = local_name(
            tag.trim_start_matches('/')
                .split_whitespace()
                .next()
                .unwrap_or(""),
        )
        .to_ascii_lowercase();
        rest = &rest[end + 1..];
        if let Some(blocked) = &skip {
            if closing && &name == blocked {
                skip = None;
            }
            continue;
        }
        match name.as_str() {
            "script" | "style" | "head" if !closing => {
                flush(&mut blocks, &mut text, heading);
                heading = false;
                skip = Some(name);
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" if !closing => {
                flush(&mut blocks, &mut text, heading);
                heading = true;
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                flush(&mut blocks, &mut text, true);
                heading = false;
            }
            "p" | "div" | "li" | "tr" | "br" | "hr" | "section" | "article" | "blockquote"
            | "pre"
                if !closing =>
            {
                flush(&mut blocks, &mut text, heading);
                heading = false;
            }
            "p" | "div" | "li" | "tr" | "section" | "article" | "blockquote" | "pre" => {
                flush(&mut blocks, &mut text, heading);
                heading = false;
            }
            _ => {}
        }
    }
    if skip.is_none() {
        let chunk = decode_entities(rest);
        text.push_str(&chunk);
        flush(&mut blocks, &mut text, heading);
    }
    blocks
}

fn flush(blocks: &mut Vec<Block>, text: &mut String, heading: bool) {
    let value = normalize_text(text);
    text.clear();
    let value = value.trim();
    if !value.is_empty() {
        blocks.push(Block {
            heading,
            text: value.to_owned(),
        });
    }
}

fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(index) = rest.find('&') {
        out.push_str(&rest[..index]);
        let Some(end) = rest[index..].find(';') else {
            out.push_str(&rest[index..]);
            return out;
        };
        let entity = &rest[index + 1..index + end];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" | "#39" => Some('\''),
            "nbsp" => Some(' '),
            "mdash" | "ndash" => Some('-'),
            "hellip" => None,
            _ => {
                let code = entity
                    .strip_prefix("#x")
                    .or_else(|| entity.strip_prefix("#X"))
                    .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                    .or_else(|| entity.strip_prefix('#').and_then(|dec| dec.parse().ok()));
                code.and_then(char::from_u32)
            }
        };
        if entity == "hellip" {
            out.push_str("...");
        } else if let Some(ch) = decoded {
            out.push(match ch {
                '\u{2018}' | '\u{2019}' => '\'',
                '\u{201C}' | '\u{201D}' => '"',
                '\u{2013}' | '\u{2014}' => '-',
                other => other,
            });
        } else {
            out.push('&');
            out.push_str(entity);
            out.push(';');
        }
        rest = &rest[index + end + 1..];
    }
    out.push_str(rest);
    out
}

// ---------------------------------------------------------------- normalize

/// Canonical typography and spacing: smart quotes become straight quotes,
/// dashes become `-`, ellipses `...`, unusual spaces ordinary spaces, and
/// line/trailing spaces are trimmed. Paragraph breaks survive.
pub fn normalize_text(text: &str) -> String {
    let replaced =
        text.replace("\r\n", "\n")
            .replace('\r', "\n")
            .replace('\u{2026}', "...")
            .chars()
            .map(|ch| match ch {
                '\u{00A0}' | '\u{2007}' | '\u{2009}' | '\u{200A}' | '\u{202F}' | '\u{200B}'
                | '\t' => ' ',
                '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' => '\'',
                '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' => '"',
                '\u{2013}' | '\u{2014}' | '\u{2015}' | '\u{2212}' => '-',
                ch if ch.is_control() && ch != '\n' => ' ',
                ch => ch,
            })
            .collect::<String>();
    let mut lines = Vec::new();
    for line in replaced.split('\n') {
        let line = line.trim();
        let mut out = String::with_capacity(line.len());
        let mut space = false;
        for ch in line.chars() {
            if ch == ' ' {
                space = true;
            } else {
                if space && !out.is_empty() {
                    out.push(' ');
                }
                space = false;
                out.push(ch);
            }
        }
        lines.push(out);
    }
    let mut out = String::new();
    let mut blank = 0usize;
    for line in lines {
        if line.is_empty() {
            blank += 1;
            if blank == 1 {
                out.push('\n');
            }
        } else {
            blank = 0;
            out.push_str(&line);
            out.push('\n');
        }
    }
    out.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_quotes_spaces_and_hyphenation() {
        let (text, healed) = heal_hyphenation("a won-\nderful  cat\u{00A0}sat\r\n\r\nnext page");
        assert_eq!(healed, 1);
        let text = normalize_text(&text);
        assert_eq!(text, "a wonderful cat sat\n\nnext page");
        assert_eq!(
            normalize_text("\u{201C}Hello\u{201D} \u{2014} world\u{2026}"),
            "\"Hello\" - world..."
        );
    }

    #[test]
    fn synthesizes_chapters_when_missing() {
        let mut warnings = Vec::new();
        let blocks = vec![
            Block {
                heading: false,
                text: "First paragraph.".into(),
            },
            Block {
                heading: false,
                text: "Second paragraph.".into(),
            },
        ];
        let chapters = assemble("A Book", blocks, &mut warnings).unwrap();
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].title, "A Book");
        assert!(warnings[0].contains("synthesized"));
    }

    #[test]
    fn detects_headings_and_keeps_preamble() {
        let mut warnings = Vec::new();
        let blocks = vec![
            Block {
                heading: false,
                text: "Preface text.".into(),
            },
            Block {
                heading: true,
                text: "Chapter One".into(),
            },
            Block {
                heading: false,
                text: "The story starts.".into(),
            },
        ];
        let chapters = assemble("Book", blocks, &mut warnings).unwrap();
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, "Beginning");
    }

    #[test]
    fn strips_running_headers_and_page_numbers() {
        let mut pages = vec![
            "THE BOOK\nOne line.\n12".into(),
            "THE BOOK\nAnother line.\n13".into(),
            "THE BOOK\nFinal line.\n14".into(),
        ];
        let removed = strip_running_lines(&mut pages);
        assert_eq!(removed, 6);
        assert_eq!(pages[0], "One line.");
    }

    #[test]
    fn parses_epub_attributes_and_entities() {
        let attrs =
            parse_attrs(r#"item id="t1" href="text/one.xhtml" media-type="application/xhtml+xml""#);
        assert_eq!(attrs["href"], "text/one.xhtml");
        assert_eq!(
            decode_entities("Tom &amp; Jerry &#8212; ok"),
            "Tom & Jerry - ok"
        );
        let blocks = html_blocks("<h1>One</h1><p>Two&nbsp;words.</p>");
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].heading);
        assert_eq!(blocks[1].text, "Two words.");
    }

    #[test]
    fn renders_frontmatter_and_chapters() {
        let markdown = render(
            "book.demo",
            "A \"Demo\"",
            "",
            "A1",
            "original",
            "Somewhere",
            &[Chapter {
                title: "One".into(),
                paragraphs: vec!["Text.".into()],
            }],
        );
        assert!(markdown.starts_with("---\nid: \"book.demo\""));
        assert!(markdown.contains("title: \"A \\\"Demo\\\"\""));
        assert!(markdown.contains("## One\n\nText."));
    }
}
