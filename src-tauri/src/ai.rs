//! AI provider connections: local OpenAI-compatible servers (Ollama, LM Studio,
//! vLLM) and cloud APIs (OpenAI, Gemini, Anthropic, OpenRouter…).
//!
//! Every request leaves from Rust, so the webview never deals with CORS and the
//! API keys stay in the local database instead of browser storage.

use std::time::Duration;

use noterust_db::ai::AiConnectionDraft;
use noterust_db::AiConnection;
use serde_json::{json, Value};
use tauri::State;

use crate::commands::{db_call, AppState};

const TIMEOUT: Duration = Duration::from_secs(120);

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(TIMEOUT)
        .build()
        .map_err(|err| format!("تعذر تهيئة عميل الشبكة: {err}"))
}

fn endpoint(base: &str, path: &str) -> String {
    format!("{}{}", base.trim_end_matches('/'), path)
}

/// Human readable network error with the address that failed.
fn network_error(url: &str, err: &reqwest::Error) -> String {
    let host = url
        .split("://")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .unwrap_or(url);
    if err.is_timeout() {
        format!("انتهت المهلة قبل رد {host} — تأكد أن الخادم يعمل.")
    } else if err.is_connect() {
        format!("تعذر الاتصال بـ {host} — هل الخادم يعمل؟")
    } else {
        format!("فشل الاتصال بـ {host}: {err}")
    }
}

/// Sends a chat prompt and returns the model's plain text answer.
async fn complete(connection: &AiConnection, prompt: &str) -> Result<String, String> {
    let http = client()?;
    match connection.kind.as_str() {
        "gemini" => gemini(&http, connection, prompt).await,
        "anthropic" => anthropic(&http, connection, prompt).await,
        _ => openai(&http, connection, prompt).await,
    }
}

async fn openai(
    http: &reqwest::Client,
    connection: &AiConnection,
    prompt: &str,
) -> Result<String, String> {
    let url = endpoint(&connection.base_url, "/chat/completions");
    let mut request = http.post(&url).json(&json!({
        "model": connection.model,
        "messages": [{ "role": "user", "content": prompt }],
        "stream": false
    }));
    if !connection.api_key.is_empty() {
        request = request.bearer_auth(&connection.api_key);
    }
    let response = request
        .send()
        .await
        .map_err(|err| network_error(&url, &err))?;
    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|err| format!("رد غير مفهوم من الخادم: {err}"))?;
    if !status.is_success() {
        return Err(provider_error(status.as_u16(), &body));
    }
    body.pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "لم يُرجع الخادم نصاً".to_string())
}

async fn gemini(
    http: &reqwest::Client,
    connection: &AiConnection,
    prompt: &str,
) -> Result<String, String> {
    let base = if connection.base_url.is_empty() {
        "https://generativelanguage.googleapis.com/v1beta".to_string()
    } else {
        connection.base_url.clone()
    };
    let model = if connection.model.is_empty() {
        "gemini-2.0-flash".to_string()
    } else {
        connection.model.clone()
    };
    let url = endpoint(&base, &format!("/models/{model}:generateContent"));
    let response = http
        .post(&url)
        .query(&[("key", connection.api_key.as_str())])
        .json(&json!({ "contents": [{ "parts": [{ "text": prompt }] }] }))
        .send()
        .await
        .map_err(|err| network_error(&url, &err))?;
    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|err| format!("رد غير مفهوم من Gemini: {err}"))?;
    if !status.is_success() {
        return Err(provider_error(status.as_u16(), &body));
    }
    let text = body
        .pointer("/candidates/0/content/parts")
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    if text.is_empty() {
        return Err("لم يُرجع Gemini نصاً".to_string());
    }
    Ok(text)
}

async fn anthropic(
    http: &reqwest::Client,
    connection: &AiConnection,
    prompt: &str,
) -> Result<String, String> {
    let base = if connection.base_url.is_empty() {
        "https://api.anthropic.com".to_string()
    } else {
        connection.base_url.clone()
    };
    let model = if connection.model.is_empty() {
        "claude-sonnet-4-5".to_string()
    } else {
        connection.model.clone()
    };
    let url = endpoint(&base, "/v1/messages");
    let response = http
        .post(&url)
        .header("x-api-key", &connection.api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&json!({
            "model": model,
            "max_tokens": 2048,
            "messages": [{ "role": "user", "content": prompt }]
        }))
        .send()
        .await
        .map_err(|err| network_error(&url, &err))?;
    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|err| format!("رد غير مفهوم من Claude: {err}"))?;
    if !status.is_success() {
        return Err(provider_error(status.as_u16(), &body));
    }
    let text = body
        .get("content")
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    if text.is_empty() {
        return Err("لم يُرجع Claude نصاً".to_string());
    }
    Ok(text)
}

/// Turns a provider error body into a readable Arabic message.
fn provider_error(status: u16, body: &Value) -> String {
    let detail = body
        .pointer("/error/message")
        .and_then(Value::as_str)
        .or_else(|| body.get("message").and_then(Value::as_str))
        .or_else(|| body.pointer("/error/status").and_then(Value::as_str))
        .unwrap_or("");
    match status {
        401 | 403 => format!("المفتاح مرفوض ({status}). {detail}"),
        404 => format!("العنوان أو الموديل غير صحيح ({status}). {detail}"),
        429 => format!("تجاوزت الحد المسموح مؤقتاً ({status}). {detail}"),
        _ if detail.is_empty() => format!("رفض الخادم الطلب ({status})."),
        _ => format!("خطأ من الخادم ({status}): {detail}"),
    }
}

/// Lists the saved connections.
#[tauri::command]
pub async fn ai_connections(state: State<'_, AppState>) -> Result<Vec<AiConnection>, String> {
    let db = state.db.clone();
    db_call(db, |db| noterust_db::ai::list(db.conn())).await
}

/// Creates or updates a connection.
#[tauri::command]
pub async fn ai_save_connection(
    state: State<'_, AppState>,
    draft: AiConnectionDraft,
) -> Result<AiConnection, String> {
    let db = state.db.clone();
    db_call(db, move |db| noterust_db::ai::save(db.conn(), &draft)).await
}

/// Deletes a connection.
#[tauri::command]
pub async fn ai_delete_connection(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.clone();
    db_call(db, move |db| noterust_db::ai::delete(db.conn(), id)).await
}

/// Checks that a saved connection answers, and reports how many models it lists.
#[tauri::command]
pub async fn ai_test_connection(state: State<'_, AppState>, id: i64) -> Result<String, String> {
    let db = state.db.clone();
    let connection = db_call(db, move |db| noterust_db::ai::get(db.conn(), id)).await?;
    let http = client()?;
    let url = match connection.kind.as_str() {
        "gemini" => {
            let base = if connection.base_url.is_empty() {
                "https://generativelanguage.googleapis.com/v1beta".to_string()
            } else {
                connection.base_url.clone()
            };
            endpoint(&base, "/models")
        }
        "anthropic" => {
            let base = if connection.base_url.is_empty() {
                "https://api.anthropic.com".to_string()
            } else {
                connection.base_url.clone()
            };
            endpoint(&base, "/v1/models")
        }
        _ => endpoint(&connection.base_url, "/models"),
    };

    let mut request = http.get(&url);
    request = match connection.kind.as_str() {
        "gemini" => request.query(&[("key", connection.api_key.as_str())]),
        "anthropic" => request
            .header("x-api-key", &connection.api_key)
            .header("anthropic-version", "2023-06-01"),
        _ if connection.api_key.is_empty() => request,
        _ => request.bearer_auth(&connection.api_key),
    };

    let response = request
        .send()
        .await
        .map_err(|err| network_error(&url, &err))?;
    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|err| format!("رد غير مفهوم من الخادم: {err}"))?;
    if !status.is_success() {
        return Err(provider_error(status.as_u16(), &body));
    }
    let models = body
        .get("data")
        .or_else(|| body.get("models"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    item.get("id")
                        .or_else(|| item.get("name"))
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if models.is_empty() {
        return Ok("متصل ✅".to_string());
    }
    let preview = models
        .iter()
        .take(4)
        .cloned()
        .collect::<Vec<_>>()
        .join("، ");
    Ok(format!("متصل ✅ — {} موديل ({preview}…)", models.len()))
}

/// Sends one prompt to a saved connection and returns its answer.
#[tauri::command]
pub async fn ai_chat(
    state: State<'_, AppState>,
    id: i64,
    prompt: String,
) -> Result<String, String> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return Err("اكتب سؤالك أولاً".to_string());
    }
    let db = state.db.clone();
    let connection = db_call(db, move |db| noterust_db::ai::get(db.conn(), id)).await?;
    complete(&connection, trimmed).await
}
