use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use refl_server::access::{
    HttpTelegramGateway, NoopTelegramGateway, RegistrationMode, TelegramGateway,
};
use refl_server::routes::{AppState, Config, DEFAULT_MIN_CORE_VERSION, router};
use refl_server::telegram_bot;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = env::var("DATABASE_URL")?;
    let bind_addr: SocketAddr = match env::var("BIND_ADDR") {
        Ok(value) => value,
        Err(_) => "127.0.0.1:3000".into(),
    }
    .parse()?;
    let allow_header_learner_id = env::var("ALLOW_HEADER_LEARNER_ID")
        .is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"));
    let dev_auth =
        env::var("DEV_AUTH").is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"));
    let registration_mode =
        RegistrationMode::from_value(env::var("REGISTRATION_MODE").ok().as_deref());
    let telegram_bot_token = env::var("TELEGRAM_BOT_TOKEN")
        .ok()
        .filter(|token| !token.is_empty());
    let admin_telegram_chat_id = match env::var("ADMIN_TELEGRAM_CHAT_ID") {
        Ok(value) => Some(value.parse::<i64>()?),
        Err(_) => None,
    };
    let min_core_version = match env::var("MIN_CORE_VERSION") {
        Ok(value) => value,
        Err(_) => DEFAULT_MIN_CORE_VERSION.into(),
    };

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;
    let http_gateway = match &telegram_bot_token {
        Some(token) => Some(Arc::new(HttpTelegramGateway::new(token.clone())?)),
        None => None,
    };
    let telegram: Arc<dyn TelegramGateway> = match &http_gateway {
        Some(gateway) => gateway.clone(),
        None => Arc::new(NoopTelegramGateway),
    };
    if let (Some(gateway), Some(chat_id)) = (&http_gateway, admin_telegram_chat_id) {
        telegram_bot::spawn(pool.clone(), Arc::clone(gateway), chat_id);
    }
    let app = router(AppState {
        pool,
        config: Config {
            allow_header_learner_id,
            dev_auth,
            registration_mode,
            telegram_bot_token,
            admin_telegram_chat_id,
            min_core_version,
        },
        telegram,
    });
    let listener = TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
