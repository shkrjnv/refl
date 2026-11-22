//! Importing content bundles (`docs/02-layer1-content.md`, «Идентификаторы и
//! версии»): per-entity tables keyed by content version, idempotent by
//! version, exactly one active version.

use eng_core::bundle::{Bundle, BundleContent, KcStatus};
use eng_core::content::ExplanationTarget;
use eng_core::item::KcRole;
use serde::Serialize;
use serde_json::Value;
use sqlx::{PgConnection, Postgres, Transaction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Imported {
    New,
    /// This content version was imported before; nothing changed.
    AlreadyPresent,
}

fn json(v: &impl Serialize) -> Value {
    serde_json::to_value(v).expect("content types serialize")
}

/// The name a value has in content files, e.g. `requires`, `short`.
fn wire(v: &impl Serialize) -> String {
    match json(v) {
        Value::String(s) => s,
        other => other.to_string(),
    }
}

/// Imports `bundle` in one transaction. Does not change the active version.
///
/// # Errors
/// Database errors; the transaction is rolled back.
///
/// # Panics
/// Never: bundle types always serialize.
pub async fn import_bundle(
    tx: &mut Transaction<'_, Postgres>,
    bundle: &Bundle,
) -> Result<Imported, sqlx::Error> {
    let v = bundle.content_version.0.as_str();
    let c = &bundle.content;
    let inserted = sqlx::query(
        "INSERT INTO content_versions (version, format_version, normalization_version, tokens) \
         VALUES ($1, $2, $3, $4) ON CONFLICT (version) DO NOTHING",
    )
    .bind(v)
    .bind(i64::from(bundle.format_version))
    .bind(i64::from(c.normalization_version))
    .bind(json(&c.tokens))
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

/// KCs with status, edges and the `requires` closure.
async fn insert_graph(
    conn: &mut PgConnection,
    v: &str,
    c: &BundleContent,
) -> Result<(), sqlx::Error> {
    for k in &c.kcs {
        let status = match k.status {
            KcStatus::Draft => "draft",
            KcStatus::Live => "live",
        };
        sqlx::query("INSERT INTO content_kcs VALUES ($1, $2, $3, $4)")
            .bind(v)
            .bind(k.kc.id.as_str())
            .bind(status)
            .bind(json(&k.kc))
            .execute(&mut *conn)
            .await?;
    }
    for e in &c.edges {
        sqlx::query("INSERT INTO content_edges VALUES ($1, $2, $3, $4)")
            .bind(v)
            .bind(wire(&e.kind))
            .bind(e.from.as_str())
            .bind(e.to.as_str())
            .execute(&mut *conn)
            .await?;
    }
    for (kc, required) in &c.requires_closure {
        for r in required {
            sqlx::query("INSERT INTO content_requires_closure VALUES ($1, $2, $3)")
                .bind(v)
                .bind(kc.as_str())
                .bind(r.as_str())
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
            .bind(json(m))
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
            .bind(wire(&e.depth))
            .bind(wire(&e.lang))
            .bind(json(e))
            .execute(&mut *conn)
            .await?;
    }
    for i in &c.instructions {
        sqlx::query("INSERT INTO content_instructions VALUES ($1, $2, $3)")
            .bind(v)
            .bind(i.id.as_str())
            .bind(json(i))
            .execute(&mut *conn)
            .await?;
    }
    for l in &c.lessons {
        sqlx::query("INSERT INTO content_lessons VALUES ($1, $2, $3)")
            .bind(v)
            .bind(l.id.as_str())
            .bind(json(l))
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

/// Items with their automata and the item indexes by KC.
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
            .bind(wire(&item.status))
            .bind(entry.session_eligible)
            .bind(json(item))
            .bind(json(&entry.slots))
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
                    .bind(wire(&role))
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
