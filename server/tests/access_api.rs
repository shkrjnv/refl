//! Offline integration tests for Telegram login, registration gates, invites,
//! admin approval, and access-status enforcement.

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use refl_server::MIGRATOR;
use refl_server::access::{MockTelegramGateway, RegistrationMode};
use refl_server::routes::{AppState, Config, router};
use refl_server::telegram_bot;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Executor, PgPool};
use tower::ServiceExt;
use uuid::Uuid;

const BOT_TOKEN: &str = "123456:TEST_TOKEN";
const ADMIN_CHAT: i64 = 777;

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
        let dir = std::env::temp_dir().join(format!("refl-pg-access-{}", std::process::id()));
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

fn test_app(
    pool: PgPool,
    mode: RegistrationMode,
    token: Option<&str>,
    gateway: Arc<MockTelegramGateway>,
) -> Router {
    router(AppState {
        pool,
        config: Config {
            allow_header_learner_id: false,
            dev_auth: true,
            registration_mode: mode,
            telegram_bot_token: token.map(str::to_owned),
            admin_telegram_chat_id: Some(ADMIN_CHAT),
            admin_email: None,
            admin_telegram_id: None,
            min_core_version: "0.0.0".into(),
        },
        telegram: gateway,
    })
}

fn test_app_with_admin(
    pool: PgPool,
    mode: RegistrationMode,
    token: Option<&str>,
    gateway: Arc<MockTelegramGateway>,
    admin_email: Option<&str>,
    admin_telegram_id: Option<i64>,
) -> Router {
    router(AppState {
        pool,
        config: Config {
            allow_header_learner_id: false,
            dev_auth: true,
            registration_mode: mode,
            telegram_bot_token: token.map(str::to_owned),
            admin_telegram_chat_id: Some(ADMIN_CHAT),
            admin_email: admin_email.map(str::to_owned),
            admin_telegram_id,
            min_core_version: "0.0.0".into(),
        },
        telegram: gateway,
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

async fn get(app: Router, uri: &str, token: Option<&str>) -> (StatusCode, Value) {
    let mut builder = Request::builder().method("GET").uri(uri);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request = builder.body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

fn code_for(id: Uuid) -> String {
    let mut code = id.simple().to_string().to_uppercase();
    code.truncate(8);
    code
}

async fn create_active_email_user(db: &PgPool, id: Uuid, email: &str, admin: bool) {
    sqlx::query(
        "INSERT INTO users \
         (id, public_code, display_name, interface_lang, access_status) \
         VALUES ($1, $2, 'Test', 'en', 'active')",
    )
    .bind(id)
    .bind(code_for(id))
    .execute(db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO login_methods (id, user_id, kind, identifier, verified_at) \
         VALUES ($1, $2, 'email', $3, now())",
    )
    .bind(Uuid::new_v4())
    .bind(id)
    .bind(email)
    .execute(db)
    .await
    .unwrap();
    sqlx::query("INSERT INTO user_roles (user_id, role) VALUES ($1, 'learner')")
        .bind(id)
        .execute(db)
        .await
        .unwrap();
    if admin {
        sqlx::query("INSERT INTO user_roles (user_id, role) VALUES ($1, 'admin')")
            .bind(id)
            .execute(db)
            .await
            .unwrap();
    }
}

async fn login_email(app: &Router, email: &str) -> (Uuid, String) {
    let (status, body) = post(
        app.clone(),
        "/v1/auth/request-code",
        json!({ "email": email }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let code = body["code"].as_str().unwrap();
    let (status, body) = post(
        app.clone(),
        "/v1/auth/verify-code",
        json!({ "email": email, "code": code }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    (
        Uuid::parse_str(body["learner_id"].as_str().unwrap()).unwrap(),
        body["access_token"].as_str().unwrap().to_owned(),
    )
}

fn now_seconds() -> i64 {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    i64::try_from(elapsed.as_secs()).unwrap()
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> Vec<u8> {
    let mut key_block = [0u8; 64];
    if key.len() > 64 {
        let digest = Sha256::digest(key);
        key_block[..32].copy_from_slice(&digest);
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }
    let mut inner_pad = [0u8; 64];
    let mut outer_pad = [0u8; 64];
    for ((inner, outer), byte) in inner_pad
        .iter_mut()
        .zip(outer_pad.iter_mut())
        .zip(key_block.iter())
    {
        *inner = byte ^ 0x36;
        *outer = byte ^ 0x5c;
    }
    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(message);
    let inner_hash = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner_hash);
    outer.finalize().to_vec()
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::new();
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn telegram_body(id: i64, auth_date: i64, invite_code: Option<&str>) -> Value {
    let first_name = "Test";
    let last_name = "User";
    let username = "testuser";
    let photo_url = "https://example.invalid/p.png";
    let data = format!(
        "auth_date={auth_date}\nfirst_name={first_name}\nid={id}\nlast_name={last_name}\nphoto_url={photo_url}\nusername={username}"
    );
    let secret = Sha256::digest(BOT_TOKEN.as_bytes());
    let hash = hex(&hmac_sha256(&secret, data.as_bytes()));
    let mut body = json!({
        "id": id,
        "first_name": first_name,
        "last_name": last_name,
        "username": username,
        "photo_url": photo_url,
        "auth_date": auth_date,
        "hash": hash,
    });
    if let Some(code) = invite_code {
        body["invite_code"] = json!(code);
    }
    body
}

fn web_app_init_data(id: i64, auth_date: i64) -> String {
    let user = json!({
        "id": id,
        "first_name": "Web",
        "last_name": "App",
        "username": "webapp",
    })
    .to_string();
    let mut fields = vec![
        ("auth_date".to_owned(), auth_date.to_string()),
        ("query_id".to_owned(), "AAH-test".to_owned()),
        (
            "signature".to_owned(),
            "independent-ed25519-signature".to_owned(),
        ),
        ("user".to_owned(), user),
    ];
    fields.sort_by(|left, right| left.0.cmp(&right.0));
    let data_check = fields
        .iter()
        .filter(|(key, _)| key != "signature")
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n");
    let secret = hmac_sha256(b"WebAppData", BOT_TOKEN.as_bytes());
    let hash = hex(&hmac_sha256(&secret, data_check.as_bytes()));
    let mut serializer = form_urlencoded::Serializer::new(String::new());
    for (key, value) in &fields {
        serializer.append_pair(key, value);
    }
    serializer.append_pair("hash", &hash);
    serializer.finish()
}

fn web_app_body(init_data: &str, auth_date: i64) -> Value {
    json!({
        "id": 999_999,
        "first_name": "Ignored",
        "auth_date": auth_date,
        "hash": "0".repeat(64),
        "init_data": init_data,
    })
}

async fn telegram_hmac_and_freshness(c: &Cluster) {
    let db = migrated(c, "access_hmac").await;
    let gateway = Arc::new(MockTelegramGateway::default());
    let app = test_app(
        db.clone(),
        RegistrationMode::Open,
        Some(BOT_TOKEN),
        gateway.clone(),
    );
    let now = now_seconds();

    let (status, body) = post(
        app.clone(),
        "/v1/auth/telegram",
        telegram_body(10_001, now, None),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["access_token"].is_string());
    assert!(body["refresh_token"].is_string());

    let mut tampered = telegram_body(10_002, now, None);
    tampered["hash"] = json!("0000000000000000000000000000000000000000000000000000000000000000");
    let (status, _) = post(app.clone(), "/v1/auth/telegram", tampered, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = post(
        app,
        "/v1/auth/telegram",
        telegram_body(10_003, now - 301, None),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    db.close().await;
}

async fn telegram_not_configured(c: &Cluster) {
    let db = migrated(c, "access_telegram_missing").await;
    let gateway = Arc::new(MockTelegramGateway::default());
    let app = test_app(db.clone(), RegistrationMode::Open, None, gateway);
    let (status, body) = post(
        app,
        "/v1/auth/telegram",
        telegram_body(30_001, now_seconds(), None),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"], json!("service_unavailable"));
    db.close().await;
}

async fn telegram_web_app_init_data(c: &Cluster) {
    let db = migrated(c, "access_webapp").await;
    let gateway = Arc::new(MockTelegramGateway::default());
    let app = test_app(db.clone(), RegistrationMode::Open, Some(BOT_TOKEN), gateway);
    let now = now_seconds();

    // The claimed fields are deliberately wrong: only the verified initData
    // identity may be used.
    let (status, body) = post(
        app.clone(),
        "/v1/auth/telegram",
        web_app_body(&web_app_init_data(20_001, now), now),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let learner_id = Uuid::parse_str(body["learner_id"].as_str().unwrap()).unwrap();
    let identifier: String = sqlx::query_scalar(
        "SELECT identifier FROM login_methods \
         WHERE user_id = $1 AND kind = 'telegram'",
    )
    .bind(learner_id)
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(identifier, "20001");

    let mut tampered = web_app_init_data(20_002, now);
    tampered = tampered.replace(&format!("auth_date={now}"), "auth_date=1");
    let (status, _) = post(
        app.clone(),
        "/v1/auth/telegram",
        web_app_body(&tampered, now),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = post(
        app,
        "/v1/auth/telegram",
        web_app_body(&web_app_init_data(20_003, now - 86_401), now),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    db.close().await;
}

async fn bot_approval_callback_activates_and_notifies(c: &Cluster) {
    let db = migrated(c, "access_bot_callback").await;
    let gateway = Arc::new(MockTelegramGateway::default());
    let pending_id = Uuid::from_u128(0x0193_0000_0000_4000_8000_0000_0000_0020);
    let telegram_chat = 55_020_i64;
    sqlx::query(
        "INSERT INTO users \
         (id, public_code, display_name, interface_lang, access_status) \
         VALUES ($1, $2, 'Pending', 'en', 'pending')",
    )
    .bind(pending_id)
    .bind(code_for(pending_id))
    .execute(&db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO login_methods (id, user_id, kind, identifier, verified_at) \
         VALUES ($1, $2, 'telegram', $3, now())",
    )
    .bind(Uuid::new_v4())
    .bind(pending_id)
    .bind(telegram_chat.to_string())
    .execute(&db)
    .await
    .unwrap();

    let update = json!({
        "update_id": 1,
        "callback_query": {
            "id": "callback-1",
            "data": format!("approve:{pending_id}"),
            "message": {
                "message_id": 42,
                "chat": { "id": ADMIN_CHAT }
            }
        }
    });
    telegram_bot::handle_update(&db, gateway.as_ref(), ADMIN_CHAT, &update).await;

    let status: String = sqlx::query_scalar("SELECT access_status FROM users WHERE id = $1")
        .bind(pending_id)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(status, "active");
    assert!(
        gateway
            .callbacks()
            .iter()
            .any(|answer| { answer.callback_id == "callback-1" && answer.text == "Approved" })
    );
    assert!(gateway.cleared_keyboards().contains(&(ADMIN_CHAT, 42)));
    assert!(
        gateway.messages().iter().any(|message| {
            message.chat_id == telegram_chat && message.text == "Access approved."
        })
    );
    db.close().await;
}

async fn closed_pending_approval_and_rejection(c: &Cluster) {
    let db = migrated(c, "access_pending").await;
    let gateway = Arc::new(MockTelegramGateway::default());
    let app = test_app(
        db.clone(),
        RegistrationMode::Closed,
        Some(BOT_TOKEN),
        gateway.clone(),
    );
    let admin_id = Uuid::from_u128(0x0193_0000_0000_4000_8000_0000_0000_0001);
    create_active_email_user(&db, admin_id, "admin@example.com", true).await;
    let (_, admin_token) = login_email(&app, "admin@example.com").await;

    let (status, body) = post(
        app.clone(),
        "/v1/auth/telegram",
        telegram_body(11_001, now_seconds(), None),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body["status"], json!("pending_approval"));
    assert!(body.get("access_token").is_none());
    let pending_id = Uuid::parse_str(body["learner_id"].as_str().unwrap()).unwrap();

    let status: String = sqlx::query_scalar("SELECT access_status FROM users WHERE id = $1")
        .bind(pending_id)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(status, "pending");

    let approval = gateway
        .messages()
        .into_iter()
        .find(|message| message.chat_id == ADMIN_CHAT)
        .expect("approval request");
    assert!(approval.text.contains("Test User"));
    let labels: Vec<&str> = approval
        .buttons
        .iter()
        .map(|button| button.text.as_str())
        .collect();
    assert_eq!(labels, ["Approve", "Reject"]);
    assert!(
        approval
            .buttons
            .iter()
            .any(|button| button.callback_data == format!("approve:{pending_id}"))
    );

    let (status, pending) = get(app.clone(), "/v1/admin/requests", Some(&admin_token)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        pending
            .as_array()
            .unwrap()
            .iter()
            .any(|user| user["user_id"] == json!(pending_id))
    );

    let (status, _) = post(
        app.clone(),
        &format!("/v1/admin/requests/{pending_id}/approve"),
        json!({}),
        Some(&admin_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        gateway
            .messages()
            .iter()
            .any(|message| message.chat_id == 11_001 && message.text == "Access approved.")
    );

    let (status, body) = post(
        app,
        "/v1/auth/telegram",
        telegram_body(11_001, now_seconds(), None),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["access_token"].is_string());
    db.close().await;
}

async fn pending_rejection(c: &Cluster) {
    let db = migrated(c, "access_rejection").await;
    let gateway = Arc::new(MockTelegramGateway::default());
    let app = test_app(
        db.clone(),
        RegistrationMode::Closed,
        Some(BOT_TOKEN),
        gateway,
    );
    let admin_id = Uuid::from_u128(0x0193_0000_0000_4000_8000_0000_0000_0004);
    create_active_email_user(&db, admin_id, "reject-admin@example.com", true).await;
    let (_, admin_token) = login_email(&app, "reject-admin@example.com").await;

    let (status, body) = post(
        app.clone(),
        "/v1/auth/telegram",
        telegram_body(11_002, now_seconds(), None),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let reject_id = Uuid::parse_str(body["learner_id"].as_str().unwrap()).unwrap();
    let (status, _) = post(
        app.clone(),
        &format!("/v1/admin/requests/{reject_id}/reject"),
        json!({}),
        Some(&admin_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = post(
        app,
        "/v1/auth/telegram",
        telegram_body(11_002, now_seconds(), None),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], json!("access_rejected"));
    db.close().await;
}

async fn single_use_invite(c: &Cluster) {
    let db = migrated(c, "access_invite").await;
    let gateway = Arc::new(MockTelegramGateway::default());
    let app = test_app(
        db.clone(),
        RegistrationMode::Closed,
        Some(BOT_TOKEN),
        gateway,
    );
    let admin_id = Uuid::from_u128(0x0193_0000_0000_4000_8000_0000_0000_0002);
    create_active_email_user(&db, admin_id, "invite-admin@example.com", true).await;
    let (_, admin_token) = login_email(&app, "invite-admin@example.com").await;

    let (status, body) = post(
        app.clone(),
        "/v1/admin/invites",
        json!({ "note": "test", "max_uses": 1, "expires_in_hours": 24 }),
        Some(&admin_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let invite_code = body["invite_code"].as_str().unwrap().to_owned();
    let invite_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();
    assert!(body["expires_at"].as_i64().is_some());
    let stored: (String, i32) =
        sqlx::query_as("SELECT code_hash, used_count FROM invites WHERE id = $1")
            .bind(invite_id)
            .fetch_one(&db)
            .await
            .unwrap();
    assert_ne!(stored.0, invite_code);
    assert_eq!(stored.0.len(), 64);
    assert_eq!(stored.1, 0);

    let (status, body) = post(
        app.clone(),
        "/v1/auth/telegram",
        telegram_body(12_001, now_seconds(), Some(&invite_code)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["access_token"].is_string());

    let used: i32 = sqlx::query_scalar("SELECT used_count FROM invites WHERE id = $1")
        .bind(invite_id)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(used, 1);

    let (status, body) = post(
        app,
        "/v1/auth/telegram",
        telegram_body(12_002, now_seconds(), Some(&invite_code)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], json!("invite_invalid"));
    db.close().await;
}

async fn admin_bootstrap_under_closed_registration(c: &Cluster) {
    let db = migrated(c, "access_admin_bootstrap").await;
    let gateway = Arc::new(MockTelegramGateway::default());
    let app = test_app_with_admin(
        db.clone(),
        RegistrationMode::Closed,
        Some(BOT_TOKEN),
        gateway,
        Some("BOSS@EXAMPLE.COM"),
        Some(19_001),
    );

    let (admin_id, _) = login_email(&app, "boss@example.com").await;
    let (status, has_admin): (String, bool) = sqlx::query_as(
        "SELECT u.access_status, EXISTS( \
         SELECT 1 FROM user_roles ur WHERE ur.user_id = u.id AND ur.role = 'admin') \
         FROM users u WHERE u.id = $1",
    )
    .bind(admin_id)
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(status, "active");
    assert!(has_admin);

    let (same_id, _) = login_email(&app, "boss@example.com").await;
    assert_eq!(same_id, admin_id);
    let admin_roles: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM user_roles WHERE user_id = $1 AND role = 'admin'")
            .bind(admin_id)
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(admin_roles, 1);

    let (status, body) = post(
        app.clone(),
        "/v1/auth/telegram",
        telegram_body(19_001, now_seconds(), None),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let telegram_id = Uuid::parse_str(body["learner_id"].as_str().unwrap()).unwrap();
    let (status, has_admin): (String, bool) = sqlx::query_as(
        "SELECT u.access_status, EXISTS( \
         SELECT 1 FROM user_roles ur WHERE ur.user_id = u.id AND ur.role = 'admin') \
         FROM users u WHERE u.id = $1",
    )
    .bind(telegram_id)
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(status, "active");
    assert!(has_admin);
    db.close().await;
}

async fn open_mode_and_non_admin(c: &Cluster) {
    let db = migrated(c, "access_open").await;
    let gateway = Arc::new(MockTelegramGateway::default());
    let app = test_app(db.clone(), RegistrationMode::Open, Some(BOT_TOKEN), gateway);
    let user_id = Uuid::from_u128(0x0193_0000_0000_4000_8000_0000_0000_0003);
    create_active_email_user(&db, user_id, "plain@example.com", false).await;
    let (_, user_token) = login_email(&app, "plain@example.com").await;
    let (status, _) = get(app.clone(), "/v1/admin/requests", Some(&user_token)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, body) = post(
        app,
        "/v1/auth/telegram",
        telegram_body(13_001, now_seconds(), None),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["access_token"].is_string());
    db.close().await;
}

#[test]
fn access_api() {
    let cluster = Cluster::start();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        telegram_hmac_and_freshness(&cluster).await;
        telegram_not_configured(&cluster).await;
        telegram_web_app_init_data(&cluster).await;
        bot_approval_callback_activates_and_notifies(&cluster).await;
        closed_pending_approval_and_rejection(&cluster).await;
        pending_rejection(&cluster).await;
        single_use_invite(&cluster).await;
        admin_bootstrap_under_closed_registration(&cluster).await;
        open_mode_and_non_admin(&cluster).await;
    });
}
