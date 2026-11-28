//! HTTP integration tests against the same throwaway PostgreSQL cluster pattern
//! as `db.rs`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use refl_core::CURRENT_SCHEMA_VERSION;
use refl_core::value::Timestamp;
use refl_server::MIGRATOR;
use refl_server::access::{MockTelegramGateway, RegistrationMode};
use refl_server::content::{activate, import_bundle, import_lexicon};
use refl_server::events::insert_event;
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
        let dir = std::env::temp_dir().join(format!("refl-pg-sync-{}", std::process::id()));
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

const OTHER: Uuid = Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_0002);

fn event_id(n: u128) -> Uuid {
    Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_0000 | n)
}

fn now_ms() -> i64 {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    i64::try_from(elapsed.as_millis()).unwrap()
}

async fn migrated(c: &Cluster, name: &str) -> PgPool {
    let db = c.fresh_db(name).await;
    MIGRATOR.run(&db).await.unwrap();
    db
}

fn event(id: Uuid, learner: Uuid, client_ts: i64) -> Value {
    json!({
        "schema_version": CURRENT_SCHEMA_VERSION,
        "event_id": id,
        "learner_id": learner,
        "session_id": null,
        "client_ts": client_ts,
        "server_ts": null,
        "payload": { "type": "lesson_started", "lesson": "lesson.articles" },
    })
}

fn sync_body(learner: Uuid, events: &[Value], cursor: i64) -> Value {
    json!({
        "learner_id": learner,
        "core_version": "0.0.0",
        "content_version": "test",
        "cursor": cursor,
        "events": events,
    })
}

fn test_app(pool: PgPool) -> Router {
    router(AppState {
        pool,
        config: Config {
            allow_header_learner_id: false,
            dev_auth: true,
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

async fn get(app: Router, uri: &str) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn login(app: &Router, email: &str) -> (Uuid, String, String) {
    let (status, response) = post(
        app.clone(),
        "/v1/auth/request-code",
        json!({ "email": email }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let code = response["code"].as_str().unwrap().to_owned();
    let (status, response) = post(
        app.clone(),
        "/v1/auth/verify-code",
        json!({ "email": email, "code": code }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let learner = Uuid::parse_str(response["learner_id"].as_str().unwrap()).unwrap();
    let access = response["access_token"].as_str().unwrap().to_owned();
    let refresh = response["refresh_token"].as_str().unwrap().to_owned();
    (learner, access, refresh)
}

async fn insert_one(db: &PgPool, value: &Value) {
    let mut tx = db.begin().await.unwrap();
    insert_event(&mut tx, &value.to_string(), Timestamp(now_ms()))
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut out, byte| {
            use std::fmt::Write as _;
            let _ = write!(out, "{byte:02x}");
            out
        })
}

async fn push_is_idempotent(c: &Cluster) {
    let db = migrated(c, "sync_push").await;
    let app = test_app(db.clone());
    let (learner, access, _refresh) = login(&app, "push@example.com").await;
    let body = sync_body(learner, &[event(event_id(1), learner, now_ms())], 0);

    let (status, response) = post(app.clone(), "/v1/sync", body.clone(), Some(&access)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["accepted"], json!([event_id(1).to_string()]));
    assert_eq!(response["rejected"], json!([]));
    assert_eq!(response["events"].as_array().unwrap().len(), 1);
    assert_eq!(response["more"], json!(false));
    assert!(response["cursor"].as_i64().unwrap() > 0);
    assert!(response["server_time"].as_i64().unwrap() > 0);
    assert_eq!(response["latest_content_version"], Value::Null);

    let (status, response) = post(app, "/v1/sync", body, Some(&access)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["accepted"], json!([event_id(1).to_string()]));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM events WHERE learner_id = $1")
        .bind(learner)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(count, 1);
    db.close().await;
}

async fn placement_event_round_trips(c: &Cluster) {
    let db = migrated(c, "sync_placement").await;
    let app = test_app(db.clone());
    let (learner, access, _refresh) = login(&app, "placement@example.com").await;
    let mut placement = event(event_id(1), learner, now_ms());
    placement["payload"] = json!({
        "type": "placement_completed",
        "start_level": { "en": 1.0 },
        "answered": 4,
        "source": "diagnostic",
    });
    let (status, response) = post(
        app.clone(),
        "/v1/sync",
        sync_body(learner, &[placement], 0),
        Some(&access),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["accepted"], json!([event_id(1).to_string()]));
    assert_eq!(response["rejected"], json!([]));

    let (status, pulled) = post(app, "/v1/sync", sync_body(learner, &[], 0), Some(&access)).await;
    assert_eq!(status, StatusCode::OK);
    let events = pulled["events"].as_array().unwrap();
    let stored = events
        .iter()
        .find(|e| e["event_id"] == json!(event_id(1).to_string()))
        .expect("the placement event is pulled");
    assert_eq!(stored["payload"]["type"], json!("placement_completed"));
    assert_eq!(stored["payload"]["start_level"]["en"], json!(1.0));
    assert_eq!(stored["payload"]["source"], json!("diagnostic"));
    db.close().await;
}

async fn sync_requires_access_token(c: &Cluster) {
    let db = migrated(c, "sync_auth").await;
    let app = test_app(db.clone());
    let body = sync_body(OTHER, &[], 0);
    let (status, _) = post(app, "/v1/sync", body, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    db.close().await;
}

async fn old_core_is_rejected(c: &Cluster) {
    let db = migrated(c, "sync_old_core").await;
    let app = test_app(db.clone());
    let (learner, access, _refresh) = login(&app, "old-core@example.com").await;
    let app = router(AppState {
        pool: db.clone(),
        config: Config {
            allow_header_learner_id: false,
            dev_auth: true,
            registration_mode: RegistrationMode::Open,
            telegram_bot_token: None,
            admin_telegram_chat_id: None,
            admin_email: None,
            admin_telegram_id: None,
            min_core_version: "1.0.0".into(),
        },
        telegram: Arc::new(MockTelegramGateway::default()),
    });
    let (status, response) = post(app, "/v1/sync", sync_body(learner, &[], 0), Some(&access)).await;
    assert_eq!(status, StatusCode::UPGRADE_REQUIRED);
    assert_eq!(response["min_core_version"], json!("1.0.0"));
    db.close().await;
}

async fn batch_limit_and_future_events(c: &Cluster) {
    let db = migrated(c, "sync_reject").await;
    let app = test_app(db.clone());
    let (learner, access, _refresh) = login(&app, "reject@example.com").await;

    let too_many_events = vec![json!({}); 501];
    let too_many = sync_body(learner, &too_many_events, 0);
    let (status, _) = post(app.clone(), "/v1/sync", too_many, Some(&access)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let now = now_ms();
    let body = sync_body(
        learner,
        &[
            event(event_id(3), OTHER, now),
            event(event_id(2), learner, now + 25 * 60 * 60 * 1000),
            event(event_id(1), learner, now),
        ],
        0,
    );
    let (status, response) = post(app, "/v1/sync", body, Some(&access)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["accepted"], json!([event_id(1).to_string()]));
    let rejected = response["rejected"].as_array().unwrap();
    assert_eq!(rejected.len(), 2);
    assert_eq!(rejected[0]["event_id"], json!(event_id(2).to_string()));
    assert!(rejected[0]["reason"].as_str().unwrap().contains("future"));
    assert_eq!(rejected[1]["event_id"], json!(event_id(3).to_string()));
    assert_eq!(rejected[1]["reason"], json!("learner_mismatch"));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM events WHERE learner_id = $1")
        .bind(learner)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(count, 1);
    db.close().await;
}

async fn pull_paginates_by_server_seq(c: &Cluster) {
    let db = migrated(c, "sync_pull").await;
    let app = test_app(db.clone());
    let (learner, access, _refresh) = login(&app, "pull@example.com").await;
    let now = now_ms();
    for n in 1..=501 {
        insert_one(&db, &event(event_id(n), learner, now)).await;
    }

    let (status, response) = post(
        app.clone(),
        "/v1/sync",
        sync_body(learner, &[], 0),
        Some(&access),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["events"].as_array().unwrap().len(), 500);
    assert_eq!(response["more"], json!(true));
    assert_eq!(response["cursor"], json!(500));
    assert_eq!(
        response["events"][0]["event_id"],
        json!(event_id(1).to_string())
    );
    assert_eq!(
        response["events"][499]["event_id"],
        json!(event_id(500).to_string())
    );

    let (status, response) =
        post(app, "/v1/sync", sync_body(learner, &[], 500), Some(&access)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["events"].as_array().unwrap().len(), 1);
    assert_eq!(response["more"], json!(false));
    assert_eq!(response["cursor"], json!(501));
    assert_eq!(
        response["events"][0]["event_id"],
        json!(event_id(501).to_string())
    );
    db.close().await;
}

async fn active_content(c: &Cluster) {
    let db = migrated(c, "sync_content").await;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../content");
    let out = content_build::build(&content_build::Options::new(root)).unwrap();
    let bundle = out.bundle.unwrap_or_else(|| panic!("{}", out.report));
    let lexicon = out.lexicon.unwrap();
    let mut tx = db.begin().await.unwrap();
    import_lexicon(&mut tx, &lexicon).await.unwrap();
    import_bundle(&mut tx, &bundle).await.unwrap();
    activate(&mut tx, &bundle.content_version.0).await.unwrap();
    tx.commit().await.unwrap();

    let app = test_app(db.clone());
    let (status, response) = get(app.clone(), "/v1/content/active").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        response["content_version"].as_str(),
        Some(bundle.content_version.0.as_str())
    );
    assert_eq!(response["min_core_version"], json!("0.0.0"));
    assert_eq!(
        response["bundle"]["format_version"],
        json!(bundle.format_version)
    );
    assert_eq!(
        response["bundle"]["content_version"].as_str(),
        Some(bundle.content_version.0.as_str())
    );
    let bundle_json = response["bundle_json"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(bundle_json).unwrap();
    assert_eq!(parsed, response["bundle"]);
    assert_eq!(
        response["digest"],
        json!(sha256_hex(bundle_json.as_bytes()))
    );

    let version = bundle.content.lexicon.version.clone();
    let (status, lexicon_response) =
        get(app.clone(), &format!("/v1/content/lexicon/{version}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(lexicon_response["version"], json!(version));
    assert_eq!(lexicon_response["sha256"], json!(version));
    assert_eq!(lexicon_response["words"], json!(lexicon.len()));
    assert_eq!(lexicon_response["text"].as_str(), Some(lexicon.as_str()));
    assert_eq!(
        lexicon_response["bytes"].as_u64(),
        Some(u64::try_from(lexicon.as_str().len()).unwrap())
    );
    assert!(
        lexicon_response["license"]
            .as_str()
            .unwrap()
            .contains("SCOWL")
    );

    let (status, _) = get(
        app.clone(),
        "/v1/content/lexicon/0000000000000000000000000000000000000000000000000000000000000000",
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let empty = migrated(c, "sync_content_empty").await;
    let (status, _) = get(test_app(empty.clone()), "/v1/content/active").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    empty.close().await;
    db.close().await;
}

async fn health(c: &Cluster) {
    let db = migrated(c, "sync_health").await;
    let (status, response) = get(test_app(db.clone()), "/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response, json!({ "status": "ok" }));
    db.close().await;
}

#[test]
fn sync_api() {
    let cluster = Cluster::start();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        push_is_idempotent(&cluster).await;
        placement_event_round_trips(&cluster).await;
        sync_requires_access_token(&cluster).await;
        old_core_is_rejected(&cluster).await;
        batch_limit_and_future_events(&cluster).await;
        pull_paginates_by_server_seq(&cluster).await;
        active_content(&cluster).await;
        health(&cluster).await;
    });
}
