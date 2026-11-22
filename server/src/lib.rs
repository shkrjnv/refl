//! refl server. Stage 0 holds only the database: migrations, writing to the
//! event log and importing content bundles. No HTTP endpoints yet.

pub mod content;
pub mod events;

/// All schema migrations, embedded at compile time; each one is reversible.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");
