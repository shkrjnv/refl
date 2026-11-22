//! Database tests against a throwaway PostgreSQL cluster: `initdb` in the
//! temp directory ($TMPDIR), a server on a Unix socket only, one fresh
//! database per scenario, and `pg_ctl stop` when the test ends (also on
//! panic). Needs `initdb`, `pg_ctl` and `nickel` on PATH; the dev shell and
//! `nix flake check` provide them.
//!
//! All scenarios run inside one #[test] so that the cluster is started and
//! stopped exactly once.

use std::path::{Path, PathBuf};
use std::process::Command;

use eng_core::value::Timestamp;
use eng_server::MIGRATOR;
use eng_server::content::{Imported, activate, import_bundle};
use eng_server::events::{InsertError, Inserted, insert_event, learner_events};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Executor, PgPool, Row};
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
        let dir = std::env::temp_dir().join(format!("refl-pg-{}", std::process::id()));
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

    fn socket(&self) -> &Path {
        &self.dir
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

    fn options(&self, db: &str) -> PgConnectOptions {
        PgConnectOptions::new()
            .socket(self.socket())
            .username("postgres")
            .database(db)
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

const LEARNER: Uuid = Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_0001);
const ADMIN: Uuid = Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_0002);

/// A version-7 UUID with the given low bits.
fn event_id(n: u128) -> Uuid {
    Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_0000 | n)
}

async fn user(db: &PgPool, id: Uuid, code: &str, minor: bool) {
    sqlx::query(
        "INSERT INTO users (id, public_code, display_name, interface_lang, minor) \
         VALUES ($1, $2, 'test', 'ru', $3)",
    )
    .bind(id)
    .bind(code)
    .bind(minor)
    .execute(db)
    .await
    .unwrap();
}

fn event(id: Uuid, payload: &serde_json::Value) -> String {
    serde_json::json!({
        "schema_version": 1,
        "event_id": id,
        "learner_id": LEARNER,
        "session_id": null,
        "client_ts": 1_700_000_000_000_i64,
        "server_ts": null,
        "payload": payload,
    })
    .to_string()
}

fn lesson_started() -> serde_json::Value {
    serde_json::json!({ "type": "lesson_started", "lesson": "lesson.articles" })
}

const NOW: Timestamp = Timestamp(1_700_000_000_500);

async fn tables(db: &PgPool) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT tablename::text FROM pg_tables WHERE schemaname = 'public' \
         AND tablename <> '_sqlx_migrations' ORDER BY 1",
    )
    .fetch_all(db)
    .await
    .unwrap()
}

async fn migrations_apply_and_revert(c: &Cluster) {
    let db = c.fresh_db("migrations").await;
    MIGRATOR.run(&db).await.unwrap();
    let all = tables(&db).await;
    for t in [
        "events",
        "model_snapshots",
        "users",
        "teams",
        "notification_outbox",
        "content_items",
    ] {
        assert!(all.iter().any(|x| x == t), "missing table {t}: {all:?}");
    }
    MIGRATOR.undo(&db, 0).await.unwrap();
    assert_eq!(
        tables(&db).await,
        Vec::<String>::new(),
        "down migrations leave tables"
    );
    let leftovers: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace \
         WHERE n.nspname = 'public'",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(leftovers, 0, "down migrations leave functions");
    MIGRATOR.run(&db).await.unwrap();
    db.close().await;
}

async fn migrated(c: &Cluster, name: &str) -> PgPool {
    let db = c.fresh_db(name).await;
    MIGRATOR.run(&db).await.unwrap();
    user(&db, LEARNER, "AAAA0001", false).await;
    user(&db, ADMIN, "AAAA0002", false).await;
    db
}

async fn log_is_append_only(c: &Cluster) {
    let db = migrated(c, "append_only").await;
    insert_event(&db, &event(event_id(1), &lesson_started()), NOW)
        .await
        .unwrap();
    for stmt in [
        "UPDATE events SET client_ts = 0",
        "DELETE FROM events",
        "TRUNCATE events",
    ] {
        let err = db.execute(stmt).await.unwrap_err();
        assert!(err.to_string().contains("append-only"), "{stmt}: {err}");
    }
    // The application role cannot even attempt it.
    let mut conn = db.acquire().await.unwrap();
    conn.execute("SET ROLE refl_app").await.unwrap();
    let err = conn.execute("DELETE FROM events").await.unwrap_err();
    assert!(err.to_string().contains("permission denied"), "{err}");
    let err = conn
        .execute(format!("SELECT erase_learner('{LEARNER}')").as_str())
        .await
        .unwrap_err();
    assert!(err.to_string().contains("permission denied"), "{err}");
    conn.execute("RESET ROLE").await.unwrap();
    drop(conn);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM events")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(n, 1);
    db.close().await;
}

async fn duplicate_event_id_is_ignored(c: &Cluster) {
    let db = migrated(c, "idempotent").await;
    let e = event(event_id(1), &lesson_started());
    assert_eq!(insert_event(&db, &e, NOW).await.unwrap(), Inserted::New);
    assert_eq!(
        insert_event(&db, &e, NOW).await.unwrap(),
        Inserted::Duplicate
    );
    let events = learner_events(&db, LEARNER).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].server_ts, Some(NOW));
    db.close().await;
}

async fn invalid_events_are_rejected(c: &Cluster) {
    let db = migrated(c, "invalid").await;
    let bad_payload = event(event_id(1), &serde_json::json!({ "type": "no_such_event" }));
    assert!(matches!(
        insert_event(&db, &bad_payload, NOW).await,
        Err(InsertError::Invalid(_))
    ));
    let mut with_server_ts: serde_json::Value =
        serde_json::from_str(&event(event_id(2), &lesson_started())).unwrap();
    with_server_ts["server_ts"] = 1.into();
    assert!(matches!(
        insert_event(&db, &with_server_ts.to_string(), NOW).await,
        Err(InsertError::ServerTsSet)
    ));
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM events")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(n, 0);
    db.close().await;
}

async fn duplicate_derived_event_is_rejected(c: &Cluster) {
    let db = migrated(c, "derived").await;
    let milestone = serde_json::json!({
        "type": "milestone_reached",
        "milestone": { "kind": "level", "key": "B1" },
    });
    insert_event(&db, &event(event_id(1), &milestone), NOW)
        .await
        .unwrap();
    assert!(matches!(
        insert_event(&db, &event(event_id(2), &milestone), NOW).await,
        Err(InsertError::DerivedDuplicate)
    ));
    let derived_unlock = serde_json::json!({
        "type": "unlock_granted", "kind": "theme", "key": "blocks",
        "source": { "kind": "achievement", "achievement": "first-step" },
    });
    insert_event(&db, &event(event_id(3), &derived_unlock), NOW)
        .await
        .unwrap();
    assert!(matches!(
        insert_event(&db, &event(event_id(4), &derived_unlock), NOW).await,
        Err(InsertError::DerivedDuplicate)
    ));
    // Admin grants and revokes are not unique: re-grant after revoke works.
    let admin_grant = serde_json::json!({
        "type": "unlock_granted", "kind": "frame", "key": "gold",
        "source": { "kind": "admin", "by": ADMIN },
    });
    let revoke = serde_json::json!({
        "type": "unlock_revoked", "kind": "frame", "key": "gold", "by": ADMIN,
    });
    for (n, p) in [(5, &admin_grant), (6, &revoke), (7, &admin_grant)] {
        assert_eq!(
            insert_event(&db, &event(event_id(n), p), NOW)
                .await
                .unwrap(),
            Inserted::New
        );
    }
    let events = learner_events(&db, LEARNER).await.unwrap();
    assert_eq!(events.len(), 5);
    db.close().await;
}

async fn erasure_removes_one_learner(c: &Cluster) {
    let db = migrated(c, "erasure").await;
    let other = Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_0003);
    user(&db, other, "AAAA0003", false).await;
    insert_event(&db, &event(event_id(1), &lesson_started()), NOW)
        .await
        .unwrap();
    let mut theirs: serde_json::Value =
        serde_json::from_str(&event(event_id(2), &lesson_started())).unwrap();
    theirs["learner_id"] = other.to_string().into();
    insert_event(&db, &theirs.to_string(), NOW).await.unwrap();
    sqlx::query(
        "INSERT INTO model_snapshots (learner_id, model_version, parameters, parameters_hash, \
         state, last_event_id) VALUES ($1, 'v1', '{}', 'h', '\\x00', $2)",
    )
    .bind(LEARNER)
    .bind(event_id(1))
    .execute(&db)
    .await
    .unwrap();

    let mut conn = db.acquire().await.unwrap();
    conn.execute("SET ROLE refl_erasure").await.unwrap();
    conn.execute(format!("SELECT erase_learner('{LEARNER}')").as_str())
        .await
        .unwrap();
    conn.execute("RESET ROLE").await.unwrap();
    drop(conn);

    let left: Vec<Uuid> = sqlx::query_scalar("SELECT learner_id FROM events")
        .fetch_all(&db)
        .await
        .unwrap();
    assert_eq!(left, [other]);
    let snapshots: i64 = sqlx::query_scalar("SELECT count(*) FROM model_snapshots")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(snapshots, 0);
    let erased: Vec<Uuid> = sqlx::query_scalar("SELECT learner_id FROM erasures")
        .fetch_all(&db)
        .await
        .unwrap();
    assert_eq!(erased, [LEARNER]);
    // The erasure setting does not outlive the function: deletes fail again.
    let err = db.execute("DELETE FROM events").await.unwrap_err();
    assert!(err.to_string().contains("append-only"), "{err}");
    db.close().await;
}

async fn privacy_defaults_depend_on_minor(c: &Cluster) {
    let db = migrated(c, "privacy").await;
    let child = Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_0004);
    user(&db, child, "AAAA0004", true).await;
    let row = |id| {
        sqlx::query(
            "SELECT profile_visibility, friend_requests_from FROM privacy_settings \
             WHERE user_id = $1",
        )
        .bind(id)
        .fetch_one(&db)
    };
    let adult = row(LEARNER).await.unwrap();
    assert_eq!(adult.get::<String, _>(0), "friends");
    assert_eq!(adult.get::<String, _>(1), "anyone");
    let minor = row(child).await.unwrap();
    assert_eq!(minor.get::<String, _>(0), "private");
    assert_eq!(minor.get::<String, _>(1), "team");
    db.close().await;
}

async fn content_bundle_imports_idempotently(c: &Cluster) {
    let db = migrated(c, "content").await;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../content");
    let out = content_build::build(&content_build::Options::new(root)).unwrap();
    let bundle = out.bundle.unwrap_or_else(|| panic!("{}", out.report));
    let mut tx = db.begin().await.unwrap();
    assert_eq!(
        import_bundle(&mut tx, &bundle).await.unwrap(),
        Imported::New
    );
    activate(&mut tx, &bundle.content_version.0).await.unwrap();
    tx.commit().await.unwrap();
    let mut tx = db.begin().await.unwrap();
    assert_eq!(
        import_bundle(&mut tx, &bundle).await.unwrap(),
        Imported::AlreadyPresent
    );
    tx.commit().await.unwrap();

    let count = |table: &str| {
        let q = format!("SELECT count(*) FROM {table}");
        let db = db.clone();
        async move {
            sqlx::query_scalar::<_, i64>(&q)
                .fetch_one(&db)
                .await
                .unwrap()
        }
    };
    let n = |len: usize| i64::try_from(len).unwrap();
    let content = &bundle.content;
    assert_eq!(count("content_kcs").await, n(content.kcs.len()));
    assert_eq!(count("content_items").await, n(content.items.len()));
    assert_eq!(
        count("content_explanations").await,
        n(content.explanations.len())
    );
    assert_eq!(
        count("content_instructions").await,
        n(content.instructions.len())
    );
    let eligible: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM content_items i JOIN content_versions v \
         ON v.version = i.content_version WHERE v.active AND i.session_eligible",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(
        eligible,
        n(content.items.iter().filter(|i| i.session_eligible).count())
    );
    // A misconception's micro-explanation resolves by key.
    let feedback: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM content_explanations WHERE target_kind = 'misconception' \
         AND target_id = 'mis.article.omission-l1' AND depth = 'short' AND lang = 'ru'",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(feedback, 1);
    db.close().await;
}

#[test]
fn database() {
    let cluster = Cluster::start();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        migrations_apply_and_revert(&cluster).await;
        log_is_append_only(&cluster).await;
        duplicate_event_id_is_ignored(&cluster).await;
        invalid_events_are_rejected(&cluster).await;
        duplicate_derived_event_is_rejected(&cluster).await;
        erasure_removes_one_learner(&cluster).await;
        privacy_defaults_depend_on_minor(&cluster).await;
        content_bundle_imports_idempotently(&cluster).await;
    });
}
