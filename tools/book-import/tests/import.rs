//! End to end: a text source becomes canonical `source.md`, and the importer
//! refuses to overwrite or to publish without a license.

use std::path::PathBuf;

use book_import::{Options, import};

fn temp(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "refl-book-import-test-{}-{n}-{tag}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn text_import_repairs_and_writes_frontmatter() {
    let dir = temp("text");
    let input = dir.join("first day.txt");
    std::fs::write(
        &input,
        "Chapter One\n\nA won-\nderful cat sat.\n\nTHE BOOK\n12\n\nChapter Two\n\nA dog naps.",
    )
    .unwrap();
    let mut opts = Options::new(&input);
    opts.out = dir.join("books");
    opts.license = "original".into();
    opts.source = Some("refl test".into());
    let imported = import(&opts).unwrap();
    assert_eq!(imported.book_id, "book.first-day");
    assert_eq!(imported.chapters, 2);
    let source = std::fs::read_to_string(&imported.path).unwrap();
    assert!(source.starts_with("---\nid: \"book.first-day\""));
    assert!(source.contains("title: \"First Day\""));
    assert!(source.contains("## Chapter One"));
    assert!(source.contains("A wonderful cat sat."));
    assert!(!source.contains("THE BOOK"));
    let second = import(&opts).unwrap_err();
    assert!(second.contains("--force"));
    opts.force = true;
    import(&opts).unwrap();
}

#[test]
fn license_is_required() {
    let dir = temp("license");
    let input = dir.join("story.txt");
    std::fs::write(&input, "A story.").unwrap();
    let mut opts = Options::new(&input);
    opts.out = dir.join("books");
    let error = import(&opts).unwrap_err();
    assert!(error.contains("--license"));
}

#[test]
fn epub_import_follows_the_spine_and_headings() {
    use std::io::Write as _;

    let dir = temp("epub");
    let epub = dir.join("story.epub");
    let file = std::fs::File::create(&epub).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let stored =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file("mimetype", stored).unwrap();
    zip.write_all(b"application/epub+zip").unwrap();
    zip.start_file("META-INF/container.xml", stored).unwrap();
    zip.write_all(
        br#"<?xml version="1.0"?><container><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#,
    )
    .unwrap();
    zip.start_file("OEBPS/content.opf", stored).unwrap();
    zip.write_all(
        br#"<?xml version="1.0"?><package><metadata><dc:title>Moon Story</dc:title><dc:creator>A Writer</dc:creator></metadata><manifest><item id="c1" href="chapter1.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="c1"/></spine></package>"#,
    )
    .unwrap();
    zip.start_file("OEBPS/chapter1.xhtml", stored).unwrap();
    zip.write_all(
        br"<html><head><title>skip</title></head><body><h1>Chapter One</h1><p>Mira sees a&nbsp;cat.</p><h1>Chapter Two</h1><p>The cat naps.</p></body></html>",
    )
    .unwrap();
    zip.finish().unwrap();

    let mut opts = Options::new(&epub);
    opts.out = dir.join("books");
    opts.license = "public-domain".into();
    opts.source = Some("Test archive".into());
    let imported = import(&opts).unwrap();
    assert_eq!(imported.book_id, "book.story");
    assert_eq!(imported.chapters, 2);
    let source = std::fs::read_to_string(&imported.path).unwrap();
    assert!(source.contains("title: \"Moon Story\""));
    assert!(source.contains("author: \"A Writer\""));
    assert!(source.contains("## Chapter One"));
    assert!(source.contains("Mira sees a cat."));
    assert!(!source.contains("skip"));
}
