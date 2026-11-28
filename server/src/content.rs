//! Importing content bundles (`docs/02-layer1-content.md`, «Идентификаторы и
//! версии»): per-entity tables keyed by content version, idempotent by
//! version, exactly one active version.

use std::collections::BTreeMap;
use std::fmt::{self, Write as _};

use refl_core::bundle::{Bundle, BundleContent, KcStatus};
use refl_core::content::ExplanationTarget;
use refl_core::item::KcRole;
use refl_core::lexicon::Lexicon;
use refl_core::model::fold_version;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgExecutor, Postgres, Transaction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Imported {
    New,
    /// This content version was imported before; nothing changed.
    AlreadyPresent,
}

fn json(v: &impl Serialize) -> Result<Value, serde_json::Error> {
    serde_json::to_value(v)
}

fn db_json(v: &impl Serialize) -> Result<Value, sqlx::Error> {
    json(v).map_err(|e| sqlx::Error::Encode(Box::new(e)))
}

/// The name a value has in content files, e.g. `requires`, `short`.
fn wire(v: &impl Serialize) -> Result<String, sqlx::Error> {
    Ok(match db_json(v)? {
        Value::String(s) => s,
        other => other.to_string(),
    })
}

/// Hex SHA-256 of a lexicon or bundle file; also the wire digest format.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

/// Stores a lexicon file under its version, the hex SHA-256 of its contents
/// (`docs/02-layer1-content.md`, «Словарь проверщика»). Idempotent; import
/// it before a bundle that references it.
///
/// # Errors
/// Database errors.
pub async fn import_lexicon(
    tx: &mut Transaction<'_, Postgres>,
    lexicon: &Lexicon,
) -> Result<Imported, sqlx::Error> {
    let version = sha256_hex(lexicon.as_str().as_bytes());
    let words = i64::try_from(lexicon.len())
        .map_err(|_| sqlx::Error::Protocol("lexicon has too many words".into()))?;
    let inserted = sqlx::query(
        "INSERT INTO content_lexicons (version, words, body) VALUES ($1, $2, $3) \
         ON CONFLICT (version) DO NOTHING",
    )
    .bind(&version)
    .bind(words)
    .bind(lexicon.as_str())
    .execute(&mut **tx)
    .await?;
    Ok(if inserted.rows_affected() == 1 {
        Imported::New
    } else {
        Imported::AlreadyPresent
    })
}

/// Why a bundle was not imported.
#[derive(Debug)]
pub enum ImportError {
    Db(sqlx::Error),
    /// The bundle could not be serialized for storage and hashing.
    Serialize(serde_json::Error),
    /// The bundle lacks the projection of an item version stored by an
    /// earlier import, as `item@version`.
    PublishedVersionsMissing(Vec<String>),
    /// The bundle changes the projection of an item version stored by an
    /// earlier import, as `item@version`.
    PublishedProjectionChanged(Vec<String>),
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Db(e) => e.fmt(f),
            Self::Serialize(e) => write!(f, "bundle does not serialize: {e}"),
            Self::PublishedVersionsMissing(v) => write!(
                f,
                "the bundle lacks published item versions: {}",
                v.join(", ")
            ),
            Self::PublishedProjectionChanged(v) => write!(
                f,
                "the bundle changes published item versions: {}",
                v.join(", ")
            ),
        }
    }
}

impl std::error::Error for ImportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Db(e) => Some(e as &(dyn std::error::Error + 'static)),
            Self::Serialize(e) => Some(e as &(dyn std::error::Error + 'static)),
            Self::PublishedVersionsMissing(_) | Self::PublishedProjectionChanged(_) => None,
        }
    }
}

impl From<sqlx::Error> for ImportError {
    fn from(e: sqlx::Error) -> Self {
        Self::Db(e)
    }
}

/// A published item version never disappears or changes (`docs/02-layer1-content.md`,
/// «Публикация»): every projection stored by an earlier import must be in
/// the bundle, unchanged. This is the guard that outlives the lock file.
async fn check_published(
    conn: &mut PgConnection,
    v: &str,
    c: &BundleContent,
) -> Result<(), ImportError> {
    let stored: Vec<(String, i32, Value)> = sqlx::query_as(
        "SELECT DISTINCT item_id, version, projection FROM content_item_versions \
         WHERE content_version <> $1 ORDER BY item_id, version",
    )
    .bind(v)
    .fetch_all(&mut *conn)
    .await?;
    let in_bundle: BTreeMap<(&str, i64), Value> = c
        .item_versions
        .iter()
        .map(|e| -> Result<_, ImportError> {
            Ok((
                (e.item.as_str(), i64::from(e.version.0)),
                json(&e.projection).map_err(ImportError::Serialize)?,
            ))
        })
        .collect::<Result<_, _>>()?;
    let (mut missing, mut changed) = (Vec::new(), Vec::new());
    for (item, version, projection) in stored {
        let name = format!("{item}@{version}");
        match in_bundle.get(&(item.as_str(), i64::from(version))) {
            None => missing.push(name),
            Some(p) if *p != projection => changed.push(name),
            Some(_) => {}
        }
    }
    missing.dedup();
    changed.dedup();
    if !missing.is_empty() {
        Err(ImportError::PublishedVersionsMissing(missing))
    } else if !changed.is_empty() {
        Err(ImportError::PublishedProjectionChanged(changed))
    } else {
        Ok(())
    }
}

/// Imports `bundle` in one transaction. Does not change the active version.
///
/// # Errors
/// [`ImportError::PublishedVersionsMissing`] or
/// [`ImportError::PublishedProjectionChanged`] if the bundle drops or changes
/// an item version an earlier import stored, or if the bundle cannot be
/// serialized for storage. Database errors; the transaction is rolled back.
/// Fails if the lexicon the bundle references was not imported.
///
/// # Panics
/// Never.
pub async fn import_bundle(
    tx: &mut Transaction<'_, Postgres>,
    bundle: &Bundle,
) -> Result<Imported, ImportError> {
    let v = bundle.content_version.0.as_str();
    let c = &bundle.content;
    check_published(&mut *tx, v, c).await?;
    let bundle_bytes = serde_json::to_vec(bundle).map_err(ImportError::Serialize)?;
    let bundle_digest = sha256_hex(&bundle_bytes);
    let bundle_json: Value =
        serde_json::from_slice(&bundle_bytes).map_err(ImportError::Serialize)?;
    let inserted = sqlx::query(
        "INSERT INTO content_versions \
         (version, format_version, normalization_version, tokens, lexicon_version, fold_version, \
          bundle, bundle_digest) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT (version) DO NOTHING",
    )
    .bind(v)
    .bind(i64::from(bundle.format_version))
    .bind(i64::from(c.normalization_version))
    .bind(db_json(&c.tokens)?)
    .bind(&c.lexicon.version)
    .bind(fold_version(bundle))
    .bind(bundle_json)
    .bind(bundle_digest)
    .execute(&mut **tx)
    .await?;
    if inserted.rows_affected() == 0 {
        return Ok(Imported::AlreadyPresent);
    }
    let conn: &mut PgConnection = tx;
    insert_graph(conn, v, c).await?;
    insert_texts(conn, v, c).await?;
    insert_items(conn, v, c).await?;
    Ok(Imported::New)
}

/// Subjects, KCs with status and level rank, edges and the `requires` closure.
async fn insert_graph(
    conn: &mut PgConnection,
    v: &str,
    c: &BundleContent,
) -> Result<(), sqlx::Error> {
    for s in &c.subjects {
        sqlx::query("INSERT INTO content_subjects VALUES ($1, $2, $3)")
            .bind(v)
            .bind(s.id.as_str())
            .bind(db_json(s)?)
            .execute(&mut *conn)
            .await?;
    }
    for k in &c.kcs {
        let status = match k.status {
            KcStatus::Draft => "draft",
            KcStatus::Live => "live",
        };
        sqlx::query(
            "INSERT INTO content_kcs (content_version, id, status, body, level_rank) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(v)
        .bind(k.kc.id.as_str())
        .bind(status)
        .bind(db_json(&k.kc)?)
        .bind(i32::from(k.level_rank))
        .execute(&mut *conn)
        .await?;
    }
    for e in &c.edges {
        sqlx::query("INSERT INTO content_edges VALUES ($1, $2, $3, $4)")
            .bind(v)
            .bind(wire(&e.kind)?)
            .bind(e.from.as_str())
            .bind(e.to.as_str())
            .execute(&mut *conn)
            .await?;
    }
    for (kc, required) in &c.requires_closure {
        for r in required {
            sqlx::query(
                "INSERT INTO content_requires_closure (content_version, kc, required_kc, distance) \
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(v)
            .bind(kc.as_str())
            .bind(r.kc.as_str())
            .bind(i32::from(r.distance))
            .execute(&mut *conn)
            .await?;
        }
    }
    Ok(())
}

/// Misconceptions, explanations, instructions and lessons.
async fn insert_texts(
    conn: &mut PgConnection,
    v: &str,
    c: &BundleContent,
) -> Result<(), sqlx::Error> {
    for m in &c.misconceptions {
        sqlx::query("INSERT INTO content_misconceptions VALUES ($1, $2, $3)")
            .bind(v)
            .bind(m.id.as_str())
            .bind(db_json(m)?)
            .execute(&mut *conn)
            .await?;
    }
    for e in &c.explanations {
        let (target_kind, target_id) = match &e.target {
            ExplanationTarget::Kc(kc) => ("kc", kc.as_str()),
            ExplanationTarget::Misconception(m) => ("misconception", m.as_str()),
        };
        sqlx::query("INSERT INTO content_explanations VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(v)
            .bind(e.id.as_str())
            .bind(target_kind)
            .bind(target_id)
            .bind(wire(&e.depth)?)
            .bind(wire(&e.lang)?)
            .bind(db_json(e)?)
            .execute(&mut *conn)
            .await?;
    }
    for i in &c.instructions {
        sqlx::query("INSERT INTO content_instructions VALUES ($1, $2, $3)")
            .bind(v)
            .bind(i.id.as_str())
            .bind(db_json(i)?)
            .execute(&mut *conn)
            .await?;
    }
    for l in &c.lessons {
        sqlx::query("INSERT INTO content_lessons VALUES ($1, $2, $3)")
            .bind(v)
            .bind(l.id.as_str())
            .bind(db_json(l)?)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

/// Items with their automata, the projections of all published item
/// versions and the item indexes by KC.
async fn insert_items(
    conn: &mut PgConnection,
    v: &str,
    c: &BundleContent,
) -> Result<(), sqlx::Error> {
    for entry in &c.items {
        let item = &entry.item;
        sqlx::query("INSERT INTO content_items VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(v)
            .bind(item.id.as_str())
            .bind(i64::from(item.version.0))
            .bind(wire(&item.status)?)
            .bind(entry.session_eligible)
            .bind(db_json(item)?)
            .bind(db_json(&entry.slots)?)
            .execute(&mut *conn)
            .await?;
    }
    for e in &c.item_versions {
        sqlx::query("INSERT INTO content_item_versions VALUES ($1, $2, $3, $4)")
            .bind(v)
            .bind(e.item.as_str())
            .bind(i64::from(e.version.0))
            .bind(db_json(&e.projection)?)
            .execute(&mut *conn)
            .await?;
    }
    for (kc, refs) in &c.items_by_kc {
        for (role, list) in [
            (KcRole::Primary, &refs.primary),
            (KcRole::Secondary, &refs.secondary),
        ] {
            for r in list {
                sqlx::query("INSERT INTO content_items_by_kc VALUES ($1, $2, $3, $4, $5)")
                    .bind(v)
                    .bind(kc.as_str())
                    .bind(wire(&role)?)
                    .bind(r.id.as_str())
                    .bind(i64::from(r.version.0))
                    .execute(&mut *conn)
                    .await?;
            }
        }
    }
    Ok(())
}

/// Makes `version` the only active content version.
///
/// # Errors
/// Database errors, including an unknown version (no row activated).
pub async fn activate(
    tx: &mut Transaction<'_, Postgres>,
    version: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE content_versions SET active = false WHERE active AND version <> $1")
        .bind(version)
        .execute(&mut **tx)
        .await?;
    let r = sqlx::query("UPDATE content_versions SET active = true WHERE version = $1")
        .bind(version)
        .execute(&mut **tx)
        .await?;
    if r.rows_affected() == 1 {
        Ok(())
    } else {
        Err(sqlx::Error::RowNotFound)
    }
}

/// The SCOWL notice served with every dictionary file (`THIRD_PARTY.md`).
pub const SCOWL_LICENSE: &str = include_str!("../../licenses/SCOWL.txt");

/// A stored checker lexicon, keyed by the SHA-256 of its exact bytes.
#[derive(Debug, Clone)]
pub struct StoredLexicon {
    pub version: String,
    pub words: i32,
    pub body: String,
}

/// The active content version and the bytes served by `GET /v1/content/active`.
#[derive(Debug, Clone)]
pub struct ActiveBundle {
    pub content_version: String,
    /// SHA-256 of the complete serialized bundle.
    pub digest: String,
    pub bundle: Value,
}

/// The lexicon file for `version`, if it was imported with a bundle.
///
/// # Errors
/// Database errors.
pub async fn lexicon(
    db: impl PgExecutor<'_>,
    version: &str,
) -> Result<Option<StoredLexicon>, sqlx::Error> {
    let row: Option<(String, i32, String)> =
        sqlx::query_as("SELECT version, words, body FROM content_lexicons WHERE version = $1")
            .bind(version)
            .fetch_optional(db)
            .await?;
    Ok(row.map(|(version, words, body)| StoredLexicon {
        version,
        words,
        body,
    }))
}

/// The active bundle, or `None` if no version is active or it predates bundle
/// storage.
///
/// # Errors
/// Database errors.
pub async fn active_bundle(db: impl PgExecutor<'_>) -> Result<Option<ActiveBundle>, sqlx::Error> {
    let row: Option<(String, Option<Value>, Option<String>)> = sqlx::query_as(
        "SELECT version, bundle, bundle_digest FROM content_versions \
         WHERE active ORDER BY imported_at DESC LIMIT 1",
    )
    .fetch_optional(db)
    .await?;
    Ok(match row {
        Some((content_version, Some(bundle), Some(digest))) => Some(ActiveBundle {
            content_version,
            digest,
            bundle,
        }),
        _ => None,
    })
}

/// The active content version, if one exists.
///
/// # Errors
/// Database errors.
pub async fn latest_content_version(
    db: impl PgExecutor<'_>,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT version FROM content_versions WHERE active LIMIT 1")
        .fetch_optional(db)
        .await
}
