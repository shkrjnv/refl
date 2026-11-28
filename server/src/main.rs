use std::env;
use std::error::Error;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use refl_core::bundle::Bundle;
use refl_core::lexicon::Lexicon;
use refl_server::MIGRATOR;
use refl_server::access::{
    HttpTelegramGateway, NoopTelegramGateway, RegistrationMode, TelegramGateway,
};
use refl_server::content;
use refl_server::routes::{AppState, Config, DEFAULT_MIN_CORE_VERSION, router};
use refl_server::telegram_bot;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.as_slice() {
        [] => serve().await,
        [command] if command.as_str() == "serve" => serve().await,
        [command, path] if command.as_str() == "import-bundle" => run_import_bundle(path).await,
        _ => Err(usage_error()),
    }
}

fn usage_error() -> Box<dyn Error> {
    Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "usage: refl-server [serve | import-bundle <bundle_path>]",
    ))
}

async fn serve() -> Result<(), Box<dyn Error>> {
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
    let admin_email = env::var("ADMIN_EMAIL")
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty());
    let admin_telegram_id = match env::var("ADMIN_TELEGRAM_ID") {
        Ok(value) if !value.trim().is_empty() => Some(value.trim().parse::<i64>()?),
        _ => None,
    };
    // A private Telegram chat id equals the user id; use it when the operator
    // configured only ADMIN_TELEGRAM_ID.
    let admin_telegram_chat_id = admin_telegram_chat_id.or(admin_telegram_id);
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
    match (&http_gateway, admin_telegram_chat_id) {
        (Some(gateway), Some(chat_id)) => {
            telegram_bot::spawn(pool.clone(), Arc::clone(gateway), chat_id);
            eprintln!("refl: Telegram bot enabled; approval chat {chat_id}");
        }
        (Some(_), None) => eprintln!(
            "refl: TELEGRAM_BOT_TOKEN is set but ADMIN_TELEGRAM_CHAT_ID is missing; approval notifications are disabled"
        ),
        (None, Some(_)) => eprintln!(
            "refl: ADMIN_TELEGRAM_CHAT_ID is set but TELEGRAM_BOT_TOKEN is missing; Telegram login and notifications are disabled"
        ),
        (None, None) => eprintln!(
            "refl: Telegram is disabled; set TELEGRAM_BOT_TOKEN and ADMIN_TELEGRAM_CHAT_ID"
        ),
    }
    let app = router(AppState {
        pool,
        config: Config {
            allow_header_learner_id,
            dev_auth,
            registration_mode,
            telegram_bot_token,
            admin_telegram_chat_id,
            admin_email,
            admin_telegram_id,
            min_core_version,
        },
        telegram,
    });
    let listener = TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn run_import_bundle(bundle_path: &str) -> Result<(), Box<dyn Error>> {
    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await?;
    MIGRATOR.run(&pool).await?;

    let source = std::fs::read_to_string(bundle_path)?;
    let bundle: Bundle = serde_json::from_str(&source)?;
    let content_version = bundle.content_version.0.clone();
    let lexicon = read_sibling_lexicon(bundle_path, &bundle.content.lexicon.version)?;

    let mut tx = pool.begin().await?;
    if let Some(lexicon) = lexicon {
        content::import_lexicon(&mut tx, &lexicon).await?;
    }
    content::import_bundle(&mut tx, &bundle).await?;
    content::activate(&mut tx, &content_version).await?;
    tx.commit().await?;

    println!("{content_version}");
    pool.close().await;
    Ok(())
}

fn read_sibling_lexicon(
    bundle_path: &str,
    version: &str,
) -> Result<Option<Lexicon>, Box<dyn Error>> {
    if version.is_empty() {
        return Ok(None);
    }
    let path = Path::new(bundle_path);
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let candidate = parent.join(format!("lexicon-{version}.txt"));
    if !candidate.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(candidate)?;
    Ok(Some(Lexicon::parse(text)?))
}
