use std::env;
use std::net::SocketAddr;

use refl_server::routes::{AppState, Config, DEFAULT_MIN_CORE_VERSION, router};
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = env::var("DATABASE_URL")?;
    let bind_addr: SocketAddr = env::var("BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3000".into())
        .parse()?;
    let allow_header_learner_id = env::var("ALLOW_HEADER_LEARNER_ID")
        .is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"));
    let min_core_version =
        env::var("MIN_CORE_VERSION").unwrap_or_else(|_| DEFAULT_MIN_CORE_VERSION.into());

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;
    let app = router(AppState {
        pool,
        config: Config {
            allow_header_learner_id,
            min_core_version,
        },
    });
    let listener = TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
