//! Email-code, token lifecycle and erasure tests against a throwaway
//! PostgreSQL cluster.

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use refl_server::MIGRATOR;
use refl_server::access::{MockTelegramGateway, RegistrationMode};
use refl_server::routes::{AppState, Config, router};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Executor, PgPool};
use tower::ServiceExt;
use uuid::Uuid;

struct Cluster {
    dir: PathBuf,
}

fn run(cmd: &mut Command) {
    let out = cmd.output().unwrap_or_else(|e| panic!("{cmd:?}: {e}"));
    assert!(
        out.status.success(),
        "{cmd:?} failed:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

impl Cluster {
    fn start() -> Self {
        let dir = std::env::temp_dir().join(format!("refl-pg-auth-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let data = dir.join("data");
        run(Command::new("initdb")
            .arg("--pgdata")
            .arg(&data)
            .args(["--username", "postgres", "--auth", "trust", "--no-sync"])
            .args(["--encoding", "UTF8", "--locale", "C"]));
        run(Command::new("pg_ctl")
            .arg("--pgdata")
            .arg(&data)
            .arg("--log")
            .arg(dir.join("log"))
            .arg("--options")
            .arg(format!(
                "-k {} -c listen_addresses= -c fsync=off",
                dir.display()
            ))
            .args(["--wait", "start"]));
        Self { dir }
    }

    fn options(&self, db: &str) -> PgConnectOptions {
        PgConnectOptions::new()
            .socket(&self.dir)
            .username("postgres")
            .database(db)
    }

    async fn fresh_db(&self, name: &str) -> PgPool {
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .connect_with(self.options("postgres"))
            .await
            .unwrap();
        admin
            .execute(format!("CREATE DATABASE {name}").as_str())
            .await
            .unwrap();
        admin.close().await;
        PgPoolOptions::new()
            .max_connections(4)
            .connect_with(self.options(name))
            .await
            .unwrap()
    }
}

impl Drop for Cluster {
    fn drop(&mut self) {
        let _ = Command::new("pg_ctl")
            .arg("--pgdata")
            .arg(self.dir.join("data"))
            .args(["--mode", "immediate", "stop"])
            .output();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

async fn migrated(c: &Cluster, name: &str) -> PgPool {
    let db = c.fresh_db(name).await;
    MIGRATOR.run(&db).await.unwrap();
    db
}

fn test_app(pool: PgPool, dev_auth: bool) -> Router {
    router(AppState {
        pool,
        config: Config {
            allow_header_learner_id: false,
            dev_auth,
            registration_mode: RegistrationMode::Open,
            telegram_bot_token: None,
            admin_telegram_chat_id: None,
            admin_email: None,
            admin_telegram_id: None,
            min_core_version: "0.0.0".into(),
        },
        telegram: Arc::new(MockTelegramGateway::default()),
    })
}

async fn post(app: Router, uri: &str, body: Value, token: Option<&str>) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request = builder.body(Body::from(body.to_string())).unwrap();
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn request_code(app: &Router, email: &str) -> (StatusCode, Value) {
    post(
        app.clone(),
        "/v1/auth/request-code",
        json!({ "email": email }),
        None,
    )
    .await
}

async fn verify_code(app: &Router, email: &str, code: &str) -> (StatusCode, Value) {
    post(
        app.clone(),
        "/v1/auth/verify-code",
        json!({ "email": email, "code": code }),
        None,
    )
    .await
}

async fn login(app: &Router, email: &str) -> (Uuid, String, String) {
    let (status, body) = request_code(app, email).await;
    assert_eq!(status, StatusCode::OK);
    let code = body["code"].as_str().unwrap();
    let (status, body) = verify_code(app, email, code).await;
    assert_eq!(status, StatusCode::OK);
    let learner = Uuid::parse_str(body["learner_id"].as_str().unwrap()).unwrap();
    let access = body["access_token"].as_str().unwrap().to_owned();
    let refresh = body["refresh_token"].as_str().unwrap().to_owned();
    (learner, access, refresh)
}

fn sync_body(learner: Uuid) -> Value {
    json!({
        "learner_id": learner,
        "core_version": "0.0.0",
        "content_version": "test",
        "cursor": 0,
        "events": [],
    })
}

fn token_hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

async fn full_flow_and_logout(c: &Cluster) {
    let db = migrated(c, "auth_flow").await;
    let app = test_app(db.clone(), true);
    let (learner, access, refresh) = login(&app, "flow@example.com").await;

    let role: String = sqlx::query_scalar("SELECT role FROM user_roles WHERE user_id = $1")
        .bind(learner)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(role, "learner");

    sqlx::query(
        "UPDATE auth_sessions SET refresh_expires_at = now() + interval '1 day' \
         WHERE refresh_token_hash = $1",
    )
    .bind(token_hash(&refresh))
    .execute(&db)
    .await
    .unwrap();
    let (status, body) = post(app.clone(), "/v1/sync", sync_body(learner), Some(&access)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["accepted"], json!([]));
    let rolling: bool = sqlx::query_scalar(
        "SELECT refresh_expires_at > now() + interval '89 days' \
         FROM auth_sessions WHERE refresh_token_hash = $1",
    )
    .bind(token_hash(&refresh))
    .fetch_one(&db)
    .await
    .unwrap();
    assert!(rolling, "sync did not extend the refresh window");

    let (status, body) = post(
        app.clone(),
        "/v1/auth/logout",
        json!({ "refresh_token": refresh.clone() }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["logged_out"], json!(true));
    let (status, _) = post(
        app,
        "/v1/auth/refresh",
        json!({ "refresh_token": refresh }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    db.close().await;
}

async fn code_rejections(c: &Cluster) {
    let db = migrated(c, "auth_codes").await;
    let app = test_app(db.clone(), true);
    let email = "codes@example.com";

    let (status, body) = request_code(&app, email).await;
    assert_eq!(status, StatusCode::OK);
    let code = body["code"].as_str().unwrap().to_owned();
    assert_eq!(code.len(), 6);
    assert!(code.bytes().all(|byte| byte.is_ascii_digit()));

    let (status, _) = request_code(&app, email).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);

    let wrong = if code == "000000" { "999999" } else { "000000" };
    let (status, _) = verify_code(&app, email, wrong).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = verify_code(&app, email, &code).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = verify_code(&app, email, &code).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let expired_email = "expired@example.com";
    let (status, body) = request_code(&app, expired_email).await;
    assert_eq!(status, StatusCode::OK);
    let expired_code = body["code"].as_str().unwrap().to_owned();
    sqlx::query(
        "UPDATE login_tokens SET created_at = now() - interval '20 minutes', \
         expires_at = now() - interval '10 minutes' WHERE email = $1",
    )
    .bind(expired_email)
    .execute(&db)
    .await
    .unwrap();
    let (status, _) = verify_code(&app, expired_email, &expired_code).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    db.close().await;
}

async fn access_expiry_and_refresh(c: &Cluster) {
    let db = migrated(c, "auth_refresh").await;
    let app = test_app(db.clone(), true);
    let (learner, access, refresh) = login(&app, "refresh@example.com").await;

    sqlx::query(
        "UPDATE auth_sessions SET created_at = now() - interval '20 minutes', \
         access_expires_at = now() - interval '1 second', \
         refresh_expires_at = now() + interval '1 day' WHERE access_token_hash = $1",
    )
    .bind(token_hash(&access))
    .execute(&db)
    .await
    .unwrap();
    let (status, _) = post(app.clone(), "/v1/sync", sync_body(learner), Some(&access)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, body) = post(
        app.clone(),
        "/v1/auth/refresh",
        json!({ "refresh_token": refresh.clone() }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["expires_in"], json!(900));
    let new_access = body["access_token"].as_str().unwrap().to_owned();
    let new_refresh = body["refresh_token"].as_str().unwrap().to_owned();
    let rolling: bool = sqlx::query_scalar(
        "SELECT refresh_expires_at > now() + interval '89 days' \
         FROM auth_sessions WHERE refresh_token_hash = $1",
    )
    .bind(token_hash(&new_refresh))
    .fetch_one(&db)
    .await
    .unwrap();
    assert!(rolling);

    let (status, _) = post(
        app.clone(),
        "/v1/auth/refresh",
        json!({ "refresh_token": refresh }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, body) = post(app, "/v1/sync", sync_body(learner), Some(&new_access)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["accepted"], json!([]));
    db.close().await;
}

async fn non_dev_never_returns_code(c: &Cluster) {
    let db = migrated(c, "auth_non_dev").await;
    let app = test_app(db.clone(), false);
    let (status, body) = request_code(&app, "quiet@example.com").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["sent"], json!(true));
    assert!(body.get("code").is_none(), "non-dev response leaked a code");
    db.close().await;
}

async fn erased_account(c: &Cluster) {
    let db = migrated(c, "auth_erased").await;
    let app = test_app(db.clone(), true);
    let (learner, access, refresh) = login(&app, "erased@example.com").await;

    sqlx::query("SELECT erase_learner($1)")
        .bind(learner)
        .execute(&db)
        .await
        .unwrap();

    let (status, body) = post(app.clone(), "/v1/sync", sync_body(learner), Some(&access)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["erased"], json!(true));
    assert_eq!(body["status"], json!("erased"));

    let (status, body) = post(
        app.clone(),
        "/v1/auth/refresh",
        json!({ "refresh_token": refresh }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], json!("erased"));

    let (status, body) = request_code(&app, "erased@example.com").await;
    assert_eq!(status, StatusCode::OK);
    let code = body["code"].as_str().unwrap();
    let (status, body) = verify_code(&app, "erased@example.com", code).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], json!("erased"));
    db.close().await;
}

#[test]
fn auth_api() {
    let cluster = Cluster::start();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        full_flow_and_logout(&cluster).await;
        code_rejections(&cluster).await;
        access_expiry_and_refresh(&cluster).await;
        non_dev_never_returns_code(&cluster).await;
        erased_account(&cluster).await;
    });
}
