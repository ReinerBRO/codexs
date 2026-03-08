use base64::engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

use crate::models::ExtractedAuth;
use crate::utils::{set_private_permissions, truncate_for_error};

pub(crate) fn read_current_codex_auth() -> Result<Value, String> {
    let path = codex_auth_path()?;
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("读取当前 Codex 认证文件失败 {}: {e}", path.display()))?;
    serde_json::from_str(&raw).map_err(|e| format!("当前 Codex 认证文件不是合法 JSON: {e}"))
}

pub(crate) fn read_current_codex_auth_optional() -> Result<Option<Value>, String> {
    let path = codex_auth_path()?;
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("读取当前 Codex 认证文件失败 {}: {e}", path.display()))?;
    let value =
        serde_json::from_str(&raw).map_err(|e| format!("当前 Codex 认证文件不是合法 JSON: {e}"))?;
    Ok(Some(value))
}

pub(crate) fn write_active_codex_auth(auth_json: &Value) -> Result<(), String> {
    let path = codex_auth_path()?;
    let parent = path
        .parent()
        .ok_or_else(|| format!("无法解析 auth 目录 {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("创建 auth 目录失败 {}: {e}", parent.display()))?;

    let serialized = serde_json::to_string_pretty(auth_json)
        .map_err(|e| format!("序列化 auth.json 失败: {e}"))?;
    fs::write(&path, serialized)
        .map_err(|e| format!("写入 auth.json 失败 {}: {e}", path.display()))?;
    set_private_permissions(&path);
    Ok(())
}

pub(crate) fn extract_auth(auth_json: &Value) -> Result<ExtractedAuth, String> {
    let mode = auth_json
        .get("auth_mode")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();

    let tokens = auth_json.get("tokens").and_then(Value::as_object);
    let tokens = match tokens {
        Some(value) => value,
        None => {
            if !mode.is_empty() && mode != "chatgpt" && mode != "chatgpt_auth_tokens" {
                return Err(
                    "当前账号不是 ChatGPT 登录模式，无法读取 Codex 用量。请先执行 codex login。"
                        .to_string(),
                );
            }
            return Err("当前未检测到 ChatGPT 登录令牌，请先执行 codex login。".to_string());
        }
    };

    let access_token = tokens
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or_else(|| "auth.json 缺少 access_token".to_string())?
        .to_string();

    let id_token = tokens
        .get("id_token")
        .and_then(Value::as_str)
        .ok_or_else(|| "auth.json 缺少 id_token".to_string())?;

    let mut account_id = tokens
        .get("account_id")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let mut email = None;
    let mut plan_type = None;

    if let Ok(claims) = decode_jwt_payload(id_token) {
        email = claims
            .get("email")
            .and_then(Value::as_str)
            .map(ToString::to_string);

        let auth_claim = claims.get("https://api.openai.com/auth");
        if account_id.is_none() {
            account_id = auth_claim
                .and_then(|value| value.get("chatgpt_account_id"))
                .and_then(Value::as_str)
                .map(ToString::to_string);
        }
        plan_type = auth_claim
            .and_then(|value| value.get("chatgpt_plan_type"))
            .and_then(Value::as_str)
            .map(ToString::to_string);
    }

    let account_id =
        account_id.ok_or_else(|| "无法从 auth.json 识别 chatgpt_account_id".to_string())?;

    Ok(ExtractedAuth {
        account_id,
        access_token,
        email,
        plan_type,
    })
}

pub(crate) fn current_auth_account_id() -> Option<String> {
    read_current_codex_auth()
        .ok()
        .and_then(|auth_json| extract_auth(&auth_json).ok().map(|auth| auth.account_id))
}

pub(crate) async fn refresh_chatgpt_auth_tokens(auth_json: &Value) -> Result<Value, String> {
    let tokens = auth_json
        .get("tokens")
        .and_then(Value::as_object)
        .ok_or_else(|| "auth.json 缺少 tokens".to_string())?;

    let refresh_token = tokens
        .get("refresh_token")
        .and_then(Value::as_str)
        .ok_or_else(|| "auth.json 缺少 refresh_token".to_string())?;
    let id_token = tokens
        .get("id_token")
        .and_then(Value::as_str)
        .ok_or_else(|| "auth.json 缺少 id_token".to_string())?;

    let claims = decode_jwt_payload(id_token)?;
    let issuer = claims
        .get("iss")
        .and_then(Value::as_str)
        .unwrap_or("https://auth.openai.com")
        .trim_end_matches('/')
        .to_string();
    let token_url = format!("{issuer}/oauth/token");

    let mut form_pairs: Vec<(&str, String)> = vec![
        ("grant_type", "refresh_token".to_string()),
        ("refresh_token", refresh_token.to_string()),
    ];
    if let Some(client_id) = extract_client_id_from_claims(&claims) {
        form_pairs.push(("client_id", client_id));
    }

    let client = reqwest::Client::builder()
        .user_agent("codexs/0.3")
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let response = client
        .post(&token_url)
        .form(&form_pairs)
        .send()
        .await
        .map_err(|e| format!("刷新登录令牌失败 {token_url}: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "刷新登录令牌失败 {token_url} -> {status}: {}",
            truncate_for_error(&body, 140)
        ));
    }

    let refreshed: RefreshedTokenPayload = response
        .json()
        .await
        .map_err(|e| format!("解析刷新令牌响应失败: {e}"))?;

    let mut updated = auth_json.clone();
    let root = updated
        .as_object_mut()
        .ok_or_else(|| "auth.json 结构异常（根节点不是对象）".to_string())?;
    let tokens = root
        .get_mut("tokens")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "auth.json 缺少 tokens".to_string())?;

    tokens.insert(
        "access_token".to_string(),
        Value::String(refreshed.access_token),
    );
    tokens.insert("id_token".to_string(), Value::String(refreshed.id_token));
    if let Some(refresh_token) = refreshed.refresh_token {
        tokens.insert("refresh_token".to_string(), Value::String(refresh_token));
    }

    Ok(updated)
}

fn codex_auth_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "无法读取 HOME 目录".to_string())?;
    Ok(home.join(".codex").join("auth.json"))
}

fn decode_jwt_payload(token: &str) -> Result<Value, String> {
    let payload = token
        .split('.')
        .nth(1)
        .ok_or_else(|| "id_token 格式无效".to_string())?;

    let decoded = URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| {
            let remainder = payload.len() % 4;
            let padded = if remainder == 0 {
                payload.to_string()
            } else {
                format!("{payload}{}", "=".repeat(4 - remainder))
            };
            URL_SAFE.decode(padded)
        })
        .map_err(|e| format!("解码 id_token 失败: {e}"))?;

    serde_json::from_slice(&decoded).map_err(|e| format!("解析 id_token payload 失败: {e}"))
}

#[derive(Debug, serde::Deserialize)]
struct RefreshedTokenPayload {
    access_token: String,
    id_token: String,
    refresh_token: Option<String>,
}

fn extract_client_id_from_claims(claims: &Value) -> Option<String> {
    let aud = claims.get("aud")?;
    match aud {
        Value::String(value) => {
            if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            }
        }
        Value::Array(items) => items.iter().find_map(|item| {
            item.as_str().and_then(|value| {
                if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                }
            })
        }),
        _ => None,
    }
}
