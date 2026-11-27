//! refl server: database migrations, the event log, content import and the
//! `/v1` HTTP API.

pub mod content;
pub mod error;
pub mod events;
pub mod routes;
pub mod sync;

pub use error::AppError;
pub use routes::{AppState, AuthenticatedLearner, Config, router};

/// All schema migrations, embedded at compile time; each one is reversible.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");
