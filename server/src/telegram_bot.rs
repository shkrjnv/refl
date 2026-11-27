//! Optional Telegram Bot long-polling worker. It is started only when both
//! `TELEGRAM_BOT_TOKEN` and `ADMIN_TELEGRAM_CHAT_ID` are configured.

use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::access::{self, AccessStatus, HttpTelegramGateway, TelegramGateway};

pub fn spawn(pool: PgPool, gateway: Arc<HttpTelegramGateway>, admin_chat_id: i64) {
    drop(tokio::spawn(run(pool, gateway, admin_chat_id)));
}

async fn run(pool: PgPool, gateway: Arc<HttpTelegramGateway>, admin_chat_id: i64) {
    let mut offset = 0i64;
    loop {
        match gateway.get_updates(offset, 25).await {
            Ok(updates) => {
                for update in &updates {
                    if let Some(id) = update.get("update_id").and_then(Value::as_i64) {
                        offset = id.saturating_add(1);
                    }
                    handle_update(&pool, &gateway, admin_chat_id, update).await;
                }
            }
            Err(_) => tokio::time::sleep(Duration::from_secs(5)).await,
        }
    }
}

async fn handle_update(
    pool: &PgPool,
    gateway: &HttpTelegramGateway,
    admin_chat_id: i64,
    update: &Value,
) {
    let callback = update.get("callback_query");
    let message = update.get("message");
    let chat_id = callback
        .and_then(|callback| callback.get("message"))
        .and_then(|message| message.get("chat"))
        .and_then(|chat| chat.get("id"))
        .and_then(Value::as_i64)
        .or_else(|| {
            message
                .and_then(|message| message.get("chat"))
                .and_then(|chat| chat.get("id"))
                .and_then(Value::as_i64)
        });
    if chat_id != Some(admin_chat_id) {
        return;
    }
    if let Some(callback) = callback {
        handle_callback(pool, gateway, callback).await;
    } else if let Some(message) = message {
        handle_message(pool, gateway, admin_chat_id, message).await;
    }
}

async fn handle_callback(pool: &PgPool, gateway: &HttpTelegramGateway, callback: &Value) {
    let callback_id = callback
        .get("id")
        .and_then(Value::as_str)
        .map_or("", |value| value);
    let data = callback
        .get("data")
        .and_then(Value::as_str)
        .map_or("", |value| value);
    let Some((action, raw_user_id)) = data.split_once(':') else {
        return;
    };
    let Ok(user_id) = Uuid::parse_str(raw_user_id) else {
        return;
    };
    let (status, answer) = match action {
        "approve" => (AccessStatus::Active, "Approved"),
        "reject" => (AccessStatus::Rejected, "Rejected"),
        _ => return,
    };
    let decided = access::decide_user(pool, user_id, status, gateway)
        .await
        .is_ok();
    let text = if decided { answer } else { "Failed" };
    let _ = gateway.answer_callback(callback_id, text).await;

    let chat_id = callback
        .get("message")
        .and_then(|message| message.get("chat"))
        .and_then(|chat| chat.get("id"))
        .and_then(Value::as_i64);
    let message_id = callback
        .get("message")
        .and_then(|message| message.get("message_id"))
        .and_then(Value::as_i64);
    if let (Some(chat_id), Some(message_id)) = (chat_id, message_id) {
        let _ = gateway.clear_inline_keyboard(chat_id, message_id).await;
    }
}

async fn handle_message(
    pool: &PgPool,
    gateway: &HttpTelegramGateway,
    admin_chat_id: i64,
    message: &Value,
) {
    let text = message
        .get("text")
        .and_then(Value::as_str)
        .map_or("", |value| value);
    let Some(rest) = text.strip_prefix("/invite") else {
        return;
    };
    if !rest.is_empty() && !rest.chars().next().is_some_and(char::is_whitespace) {
        return;
    }
    let note = rest.trim();
    let note = if note.is_empty() {
        None
    } else {
        Some(note.to_owned())
    };
    let reply = match access::create_invite(pool, None, note, 1, None).await {
        Ok(invite) => invite.invite_code,
        Err(_) => "Invite creation failed.".to_owned(),
    };
    let _ = gateway.send_message(admin_chat_id, &reply, &[]).await;
}
