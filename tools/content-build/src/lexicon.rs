//! The checker's lexicon from SCOWL (`docs/02-layer1-content.md`, «Словарь
//! проверщика»; licence in `THIRD_PARTY.md`): the `en_US` and `en_GB` lists at
//! size 60, merged, lowercased and sorted. The files come from the nixpkgs
//! `scowl` package, so the same flake.lock gives the same lexicon.

use std::fmt::Write as _;
use std::path::Path;

use refl_core::lexicon::{Lexicon, LexiconRef};
use sha2::{Digest, Sha256};

/// Lists under `share/dict` of the nixpkgs `scowl` package.
pub const SCOWL_LISTS: [&str; 2] = ["wamerican.60", "wbritish.60"];

/// Reads the SCOWL lists from `dir`. They are ISO-8859-1, one word per line.
///
/// # Errors
/// A list cannot be read.
pub fn load_scowl(dir: &Path) -> std::io::Result<Lexicon> {
    let mut words = Vec::new();
    for name in SCOWL_LISTS {
        let path = dir.join(name);
        let bytes = std::fs::read(&path).map_err(|e| {
            std::io::Error::new(e.kind(), format!("SCOWL list {}: {e}", path.display()))
        })?;
        let text: String = bytes.iter().map(|&b| char::from(b)).collect();
        words.extend(text.lines().map(str::trim).map(str::to_owned));
    }
    Ok(Lexicon::from_words(words))
}

/// Hex SHA-256 of the lexicon file: its version.
pub fn version(lexicon: &Lexicon) -> String {
    Sha256::digest(lexicon.as_str().as_bytes())
        .iter()
        .fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

/// The bundle's reference: version, word count and size.
///
/// # Panics
/// Never: `Lexicon::parse` rules out files of 4 GiB and more.
pub fn reference(lexicon: &Lexicon) -> LexiconRef {
    LexiconRef {
        version: version(lexicon),
        words: u32::try_from(lexicon.len()).expect("a lexicon file is under 4 GiB"),
        bytes: u32::try_from(lexicon.as_str().len()).expect("a lexicon file is under 4 GiB"),
    }
}

/// File name of a lexicon version next to the bundle.
pub fn file_name(r: &LexiconRef) -> String {
    format!("lexicon-{}.txt", r.version)
}
