use crate::auth::{
    current_auth_account_id, extract_auth, read_current_codex_auth,
    read_current_codex_auth_optional, refresh_chatgpt_auth_tokens, write_active_codex_auth,
};
use crate::models::{
    Account, AccountStateEntry, AccountsState, AccountsStore, AppError, AppResult,
    AvailableAccount, GeneratedAccount, GenerationProgressEvent, GenerationResult, ImportResult,
    StoredAccount as ManagedStoredAccount,
};
use crate::usage::{fetch_usage, FetchedUsage};
use crate::utils::{now_unix_seconds, set_private_permissions, short_account};
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use chrono::{DateTime, SecondsFormat, Utc};
use log::warn;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    env,
    ffi::OsStr,
    fs as stdfs,
    path::{Path, PathBuf},
    process::Output,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::{fs, process::Command};

const GENERATION_PROGRESS_EVENT: &str = "generation_progress";

#[derive(Clone, Default)]
pub struct GenerationControl {
    stop_requested: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct AppState {
    account_store_lock: Mutex<()>,
}

impl GenerationControl {
    fn reset(&self) {
        self.stop_requested.store(false, Ordering::SeqCst);
    }

    fn request_stop(&self) {
        self.stop_requested.store(true, Ordering::SeqCst);
    }

    fn should_stop(&self) -> bool {
        self.stop_requested.load(Ordering::SeqCst)
    }
}

struct StopFlagResetGuard(GenerationControl);

impl Drop for StopFlagResetGuard {
    fn drop(&mut self) {
        self.0.reset();
    }
}

struct AppPaths {
    workspace_root: Option<PathBuf>,
    data_dir: PathBuf,
    scripts_dir: PathBuf,
}

#[derive(Debug)]
struct DiscoveredCodexAccount {
    email: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct RawGeneratedToken {
    #[serde(default)]
    email: String,
    #[serde(default)]
    last_refresh: String,
    #[serde(default)]
    access_token: String,
    #[serde(default)]
    account_id: String,
    #[serde(default)]
    id_token: String,
    #[serde(default)]
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct CodexTokenFile {
    #[serde(default)]
    last_refresh: String,
    #[serde(default)]
    tokens: CodexTokenFields,
}

#[derive(Debug, Default, Deserialize)]
struct CodexTokenFields {
    #[serde(default)]
    id_token: String,
}

#[derive(Debug, Default, Deserialize)]
struct ImportScriptSummary {
    #[serde(default)]
    added: Vec<String>,
    #[serde(default)]
    skipped_existing: Vec<String>,
    #[serde(default)]
    failed_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshServer {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: String,
    pub key_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SshServersState {
    #[serde(default = "default_ssh_servers_version")]
    version: u8,
    #[serde(default)]
    servers: Vec<SshServer>,
    #[serde(default)]
    auto_sync_enabled: bool,
}

impl Default for SshServersState {
    fn default() -> Self {
        Self {
            version: default_ssh_servers_version(),
            servers: Vec::new(),
            auto_sync_enabled: false,
        }
    }
}

#[derive(Debug, Default)]
struct PendingSshHost {
    aliases: Vec<String>,
    host_name: Option<String>,
    user: Option<String>,
    port: Option<u16>,
    identity_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodexSession {
    pub pid: u32,
    pub session_id: String,
    pub tty: String,
    pub cwd: String,
}

#[derive(Debug)]
struct ParsedCodexProcess {
    pid: u32,
    tty: String,
    command: String,
}

#[tauri::command]
pub async fn start_generation(
    app: AppHandle,
    control: State<'_, GenerationControl>,
    count: u32,
) -> AppResult<GenerationResult> {
    let paths = app_paths(&app)?;
    let tokens_dir = paths.data_dir.join("tokens");
    let codex_tokens_dir = paths.data_dir.join("codex_tokens");
    let state_path = paths.data_dir.join("accounts_state.json");
    let control = control.inner().clone();
    let _stop_flag_reset_guard = StopFlagResetGuard(control.clone());

    control.reset();
    ensure_dir(&tokens_dir).await?;
    ensure_dir(&codex_tokens_dir).await?;
    ensure_state_file(&state_path).await?;
    let mut state = load_state(&state_path).await?;

    let mut result = GenerationResult {
        requested: count,
        succeeded: 0,
        failed: 0,
        stopped: false,
        accounts: Vec::new(),
        errors: Vec::new(),
    };

    for current in 1..=count {
        if control.should_stop() {
            result.stopped = true;
            break;
        }

        let before_files = list_json_files(&tokens_dir, Some("token_")).await?;
        let output =
            run_python_script(&paths, &tokens_dir, "openai_register.py", &["--once"]).await?;
        let after_files = list_json_files(&tokens_dir, Some("token_")).await?;

        if control.should_stop() {
            if let Some(path) =
                detect_newest_file(before_files.clone(), after_files.clone()).await?
            {
                if let Err(error) = fs::remove_file(&path).await {
                    warn!(
                        "停止生成时清理临时 token 失败 {}: {}",
                        path.display(),
                        error
                    );
                }
            }
            result.stopped = true;
            break;
        }

        let token_file = match detect_newest_file(before_files, after_files).await? {
            Some(path) => path,
            None => {
                result.failed += 1;
                result.errors.push(format!(
                    "第 {current} 次生成失败: {}",
                    command_output_summary(&output)
                ));
                emit_progress(&app, current, count, String::new(), None);
                continue;
            }
        };

        match convert_generated_token(&paths.data_dir, &token_file, &codex_tokens_dir).await {
            Ok(account) => {
                let imported = track_generated_account(&mut state, &account.email);
                save_state(&state_path, &state).await?;

                let emitted_account = AvailableAccount {
                    email: account.email.clone(),
                    created_at: account.created_at.clone(),
                    imported,
                };

                result.succeeded += 1;
                emit_progress(
                    &app,
                    current,
                    count,
                    account.email.clone(),
                    Some(emitted_account),
                );
                result.accounts.push(account);
            }
            Err(error) => {
                result.failed += 1;
                result
                    .errors
                    .push(format!("第 {current} 次生成失败: {error}"));
                emit_progress(&app, current, count, String::new(), None);
            }
        }
    }

    Ok(result)
}

#[tauri::command]
pub fn stop_generation(control: State<'_, GenerationControl>) -> AppResult<()> {
    control.request_stop();
    Ok(())
}

#[tauri::command]
pub async fn get_accounts(app: AppHandle) -> AppResult<Vec<AvailableAccount>> {
    let paths = app_paths(&app)?;
    let state_path = paths.data_dir.join("accounts_state.json");
    ensure_state_file(&state_path).await?;

    let state = load_state(&state_path).await?;
    let imported_lookup: HashMap<String, bool> = state
        .accounts
        .into_iter()
        .map(|entry| (normalize_email(&entry.email), entry.imported))
        .collect();

    let mut accounts: Vec<AvailableAccount> = read_codex_accounts(&paths)
        .await?
        .into_iter()
        .map(|account| AvailableAccount {
            imported: imported_lookup
                .get(&account.email)
                .copied()
                .unwrap_or(false),
            email: account.email,
            created_at: account.created_at,
        })
        .collect();

    accounts.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| left.email.cmp(&right.email))
    });

    Ok(accounts)
}

#[tauri::command]
pub async fn import_accounts(app: AppHandle, emails: Vec<String>) -> AppResult<ImportResult> {
    let requested = unique_emails(emails);
    if requested.is_empty() {
        return Ok(ImportResult {
            requested: 0,
            imported: 0,
            skipped: 0,
            failed: 0,
            emails: Vec::new(),
        });
    }

    let paths = app_paths(&app)?;
    let state_path = paths.data_dir.join("accounts_state.json");
    ensure_state_file(&state_path).await?;

    let available_accounts = read_codex_accounts(&paths).await?;
    let available_lookup: HashSet<String> = available_accounts
        .into_iter()
        .map(|account| account.email)
        .collect();

    let importable: Vec<String> = requested
        .iter()
        .filter(|email| available_lookup.contains(*email))
        .cloned()
        .collect();

    if importable.is_empty() {
        return Ok(ImportResult {
            requested: requested.len(),
            imported: 0,
            skipped: requested.len(),
            failed: 0,
            emails: Vec::new(),
        });
    }

    let mut args: Vec<&str> = vec!["--emails"];
    let owned_args: Vec<String> = importable.clone();
    let borrowed_args: Vec<&str> = owned_args.iter().map(String::as_str).collect();
    args.extend(borrowed_args);

    let output =
        run_python_script(&paths, &paths.data_dir, "import_to_codex_tools.py", &args).await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let summary = parse_import_summary(&stdout)
        .ok_or_else(|| AppError::new("导入脚本未返回 SUMMARY_JSON 结果"))?;

    let mut imported_emails = summary.added;
    imported_emails.extend(summary.skipped_existing);
    let imported_emails = unique_emails(imported_emails);

    if !imported_emails.is_empty() {
        let mut state = load_state(&state_path).await?;
        mark_accounts_imported(&mut state, &imported_emails);
        save_state(&state_path, &state).await?;
    }

    if !output.status.success() && imported_emails.is_empty() {
        return Err(AppError::new(format!(
            "导入脚本执行失败: {}",
            command_output_summary(&output)
        )));
    }

    let failed = summary.failed_files.len();
    let skipped = requested
        .len()
        .saturating_sub(imported_emails.len().saturating_add(failed));

    Ok(ImportResult {
        requested: requested.len(),
        imported: imported_emails.len(),
        skipped,
        failed,
        emails: imported_emails,
    })
}

#[tauri::command]
pub async fn list_accounts(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<Account>, String> {
    let _guard = state
        .account_store_lock
        .lock()
        .map_err(|_| "账号存储锁已损坏".to_string())?;
    let store = load_accounts_store(&app)?;
    let current_account_id = current_auth_account_id();
    let mut accounts: Vec<Account> = store
        .accounts
        .iter()
        .map(|account| account.to_account(current_account_id.as_deref()))
        .collect();

    accounts.sort_by(|left, right| {
        right
            .is_current
            .cmp(&left.is_current)
            .then_with(|| right.updated_at.cmp(&left.updated_at))
            .then_with(|| left.label.cmp(&right.label))
    });
    Ok(accounts)
}

#[tauri::command]
pub async fn add_account(
    app: AppHandle,
    state: State<'_, AppState>,
    label: String,
) -> Result<Account, String> {
    let auth_json = read_current_codex_auth()?;
    let extracted = extract_auth(&auth_json)?;
    let usage_update = resolve_usage_update(
        &auth_json,
        extracted.email.clone(),
        extracted.plan_type.clone(),
    )
    .await;

    let now = now_unix_seconds();
    let default_label = extracted
        .email
        .clone()
        .unwrap_or_else(|| format!("Codex {}", short_account(&extracted.account_id)));
    let normalized_label = normalize_account_label(&label, &default_label);
    let current_account_id = current_auth_account_id();

    let _guard = state
        .account_store_lock
        .lock()
        .map_err(|_| "账号存储锁已损坏".to_string())?;
    let mut store = load_accounts_store(&app)?;

    let account = if let Some(existing) = store
        .accounts
        .iter_mut()
        .find(|account| account.account_id == extracted.account_id)
    {
        existing.label = normalized_label;
        existing.email = usage_update
            .email
            .clone()
            .or(extracted.email.clone())
            .or(existing.email.clone());
        existing.plan_type = usage_update
            .plan_type
            .clone()
            .or(extracted.plan_type.clone())
            .or(existing.plan_type.clone());
        existing.auth_json = usage_update.auth_json.clone();
        existing.updated_at = now;
        if let Some(usage) = usage_update.usage.clone() {
            existing.usage = Some(usage);
            existing.usage_error = None;
        } else {
            existing.usage_error = usage_update.usage_error.clone();
        }
        existing.to_account(current_account_id.as_deref())
    } else {
        let stored = ManagedStoredAccount {
            id: uuid::Uuid::new_v4().to_string(),
            label: normalized_label,
            email: usage_update.email.clone().or(extracted.email.clone()),
            account_id: extracted.account_id.clone(),
            plan_type: usage_update
                .plan_type
                .clone()
                .or(extracted.plan_type.clone()),
            auth_json: usage_update.auth_json.clone(),
            added_at: now,
            updated_at: now,
            usage: usage_update.usage.clone(),
            usage_error: usage_update.usage_error.clone(),
        };
        let account = stored.to_account(current_account_id.as_deref());
        store.accounts.push(stored);
        account
    };

    save_accounts_store(&app, &store)?;
    Ok(account)
}

#[tauri::command]
pub async fn delete_account(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let _guard = state
        .account_store_lock
        .lock()
        .map_err(|_| "账号存储锁已损坏".to_string())?;
    let mut store = load_accounts_store(&app)?;
    let original_len = store.accounts.len();
    store.accounts.retain(|account| account.id != id);

    if store.accounts.len() == original_len {
        return Err("未找到要删除的账号".to_string());
    }

    save_accounts_store(&app, &store)
}

#[tauri::command]
pub async fn switch_account(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    auto_resume: bool,
) -> Result<(), String> {
    let account = {
        let _guard = state
            .account_store_lock
            .lock()
            .map_err(|_| "账号存储锁已损坏".to_string())?;
        let store = load_accounts_store(&app)?;
        store
            .accounts
            .into_iter()
            .find(|account| account.id == id)
            .ok_or_else(|| "找不到要切换的账号".to_string())?
    };

    write_active_codex_auth(&account.auth_json)?;

    // 检查是否开启了SSH自动同步
    let paths = app_paths(&app).map_err(|error| error.to_string())?;
    let ssh_state = load_ssh_servers_state(&paths)
        .await
        .map_err(|error| error.to_string())?;

    if ssh_state.auto_sync_enabled && !ssh_state.servers.is_empty() {
        // 异步同步，不阻塞切换操作
        let app_clone = app.clone();
        tokio::spawn(async move {
            let _ = sync_auth_to_ssh(app_clone).await;
        });
    }

    if auto_resume {
        // 不传入工作目录，处理所有 codex 会话
        // TODO: 未来可以让前端传入工作目录进行过滤
        terminate_and_resume_sessions(None).await?;
    }

    Ok(())
}

#[tauri::command]
pub async fn refresh_account_usage(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<Account, String> {
    let stored_account = {
        let _guard = state
            .account_store_lock
            .lock()
            .map_err(|_| "账号存储锁已损坏".to_string())?;
        let store = load_accounts_store(&app)?;
        store
            .accounts
            .into_iter()
            .find(|account| account.id == id)
            .ok_or_else(|| "未找到要刷新的账号".to_string())?
    };

    let auth_json = current_auth_json_for_account(&stored_account.account_id)
        .unwrap_or_else(|| stored_account.auth_json.clone());
    let usage_update = resolve_usage_update(
        &auth_json,
        stored_account.email.clone(),
        stored_account.plan_type.clone(),
    )
    .await;
    let current_account_id = current_auth_account_id();

    let _guard = state
        .account_store_lock
        .lock()
        .map_err(|_| "账号存储锁已损坏".to_string())?;
    let mut store = load_accounts_store(&app)?;
    let account = store
        .accounts
        .iter_mut()
        .find(|account| account.id == id)
        .ok_or_else(|| "账号已被删除，无法刷新".to_string())?;

    account.updated_at = now_unix_seconds();
    account.auth_json = usage_update.auth_json.clone();
    account.email = usage_update.email.clone().or(account.email.clone());
    account.plan_type = usage_update.plan_type.clone().or(account.plan_type.clone());
    if let Some(usage) = usage_update.usage.clone() {
        account.usage = Some(usage);
        account.usage_error = None;
    } else {
        account.usage_error = usage_update.usage_error.clone();
    }

    let account = account.to_account(current_account_id.as_deref());
    save_accounts_store(&app, &store)?;
    Ok(account)
}

#[tauri::command]
pub async fn get_current_account(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<Account>, String> {
    let current_account_id = match current_auth_account_id() {
        Some(account_id) => account_id,
        None => return Ok(None),
    };

    let _guard = state
        .account_store_lock
        .lock()
        .map_err(|_| "账号存储锁已损坏".to_string())?;
    let store = load_accounts_store(&app)?;
    Ok(store
        .accounts
        .iter()
        .find(|account| account.account_id == current_account_id)
        .map(|account| account.to_account(Some(&current_account_id))))
}

#[tauri::command]
pub async fn list_ssh_servers(app: AppHandle) -> Result<Vec<SshServer>, String> {
    let paths = app_paths(&app).map_err(|error| error.to_string())?;
    let state = load_ssh_servers_state(&paths)
        .await
        .map_err(|error| error.to_string())?;
    Ok(sort_ssh_servers(state.servers))
}

#[tauri::command]
pub async fn add_ssh_server(app: AppHandle, server: SshServer) -> Result<String, String> {
    let paths = app_paths(&app).map_err(|error| error.to_string())?;
    let mut state = load_ssh_servers_state(&paths)
        .await
        .map_err(|error| error.to_string())?;
    let mut normalized = normalize_ssh_server(server)?;

    if let Some(existing) = state
        .servers
        .iter()
        .find(|item| same_ssh_server(item, &normalized))
    {
        return Ok(existing.id.clone());
    }

    if state
        .servers
        .iter()
        .any(|item| item.name.eq_ignore_ascii_case(&normalized.name))
    {
        return Err(format!("服务器名称已存在: {}", normalized.name));
    }

    normalized.id = unique_ssh_server_id(&state.servers, Some(&normalized.id));
    let created_id = normalized.id.clone();
    state.servers.push(normalized);
    state.servers = sort_ssh_servers(state.servers);
    save_ssh_servers_state(&paths, &state)
        .await
        .map_err(|error| error.to_string())?;

    Ok(created_id)
}

#[tauri::command]
pub async fn delete_ssh_server(app: AppHandle, id: String) -> Result<(), String> {
    let paths = app_paths(&app).map_err(|error| error.to_string())?;
    let mut state = load_ssh_servers_state(&paths)
        .await
        .map_err(|error| error.to_string())?;
    let target_id = id.trim();
    let original_len = state.servers.len();
    state.servers.retain(|server| server.id != target_id);

    if state.servers.len() == original_len {
        return Err(format!("未找到服务器: {target_id}"));
    }

    save_ssh_servers_state(&paths, &state)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_ssh_auto_sync(app: AppHandle) -> Result<bool, String> {
    let paths = app_paths(&app).map_err(|error| error.to_string())?;
    let state = load_ssh_servers_state(&paths)
        .await
        .map_err(|error| error.to_string())?;
    Ok(state.auto_sync_enabled)
}

#[tauri::command]
pub async fn set_ssh_auto_sync(app: AppHandle, enabled: bool) -> Result<(), String> {
    let paths = app_paths(&app).map_err(|error| error.to_string())?;
    let mut state = load_ssh_servers_state(&paths)
        .await
        .map_err(|error| error.to_string())?;
    state.auto_sync_enabled = enabled;
    save_ssh_servers_state(&paths, &state)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn sync_auth_to_ssh(app: AppHandle) -> Result<Vec<String>, String> {
    let paths = app_paths(&app).map_err(|error| error.to_string())?;
    let state = load_ssh_servers_state(&paths)
        .await
        .map_err(|error| error.to_string())?;

    if state.servers.is_empty() {
        return Ok(Vec::new());
    }

    let auth_path = codex_auth_path()?;
    if !fs::try_exists(&auth_path)
        .await
        .map_err(|error| format!("无法检查 auth.json: {error}"))?
    {
        return Err("auth.json 不存在".to_string());
    }

    let config_path = codex_config_path()?;
    let has_config = fs::try_exists(&config_path)
        .await
        .map_err(|error| format!("无法检查 config.toml: {error}"))?;

    let mut results = Vec::new();
    for server in &state.servers {
        // 同步 auth.json
        let auth_remote = format!("{}@{}:.codex/auth.json", server.username, server.host);
        let mut auth_cmd = Command::new("scp");
        auth_cmd.arg("-P").arg(server.port.to_string());

        if let Some(key_path) = &server.key_path {
            auth_cmd.arg("-i").arg(key_path);
        }

        auth_cmd.arg(&auth_path).arg(&auth_remote);

        let auth_result = match auth_cmd.output().await {
            Ok(output) if output.status.success() => Ok(()),
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("auth.json: {}", stderr.trim()))
            }
            Err(e) => Err(format!("auth.json: {}", e)),
        };

        // 同步 config.toml（如果存在）
        let config_result = if has_config {
            let config_remote = format!("{}@{}:.codex/config.toml", server.username, server.host);
            let mut config_cmd = Command::new("scp");
            config_cmd.arg("-P").arg(server.port.to_string());

            if let Some(key_path) = &server.key_path {
                config_cmd.arg("-i").arg(key_path);
            }

            config_cmd.arg(&config_path).arg(&config_remote);

            match config_cmd.output().await {
                Ok(output) if output.status.success() => Ok(()),
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(format!("config.toml: {}", stderr.trim()))
                }
                Err(e) => Err(format!("config.toml: {}", e)),
            }
        } else {
            Ok(())
        };

        // 汇总结果
        match (auth_result, config_result) {
            (Ok(()), Ok(())) => {
                results.push(format!("✓ {}", server.name));
            }
            (Err(e1), Ok(())) => {
                results.push(format!("✗ {}: {}", server.name, e1));
            }
            (Ok(()), Err(e2)) => {
                results.push(format!("✗ {}: {}", server.name, e2));
            }
            (Err(e1), Err(e2)) => {
                results.push(format!("✗ {}: {} | {}", server.name, e1, e2));
            }
        }
    }

    Ok(results)
}



#[tauri::command]
pub async fn parse_ssh_config() -> Result<Vec<SshServer>, String> {
    let config_path = ssh_config_path()?;
    if !fs::try_exists(&config_path)
        .await
        .map_err(|error| format!("无法检查 SSH config 文件: {error}"))?
    {
        return Err(format!("未找到 SSH config 文件: {}", config_path.display()));
    }

    let content = fs::read_to_string(&config_path)
        .await
        .map_err(|error| format!("读取 SSH config 失败: {error}"))?;
    Ok(parse_ssh_config_content(&content))
}

#[tauri::command]
pub async fn find_codex_sessions() -> Result<Vec<CodexSession>, String> {
    let output = Command::new("ps")
        .args(["-ax", "-o", "pid=,tty=,command="])
        .output()
        .await
        .map_err(|error| format!("执行 ps 失败: {error}"))?;

    if !output.status.success() {
        return Err(format!("执行 ps 失败: {}", command_output_summary(&output)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut sessions = Vec::new();
    let mut seen_pids = HashSet::new();

    for line in stdout.lines() {
        let Some(process) = parse_native_codex_resume_process(line) else {
            continue;
        };

        if !seen_pids.insert(process.pid) {
            continue;
        }

        let session_id = extract_session_from_command(&process.command)
            .or_else(|| query_session_from_sqlite(process.pid).ok().flatten());

        let Some(session_id) = session_id else {
            continue;
        };

        // 获取进程的工作目录
        let cwd = get_process_cwd(process.pid).await.unwrap_or_default();

        sessions.push(CodexSession {
            pid: process.pid,
            session_id,
            tty: process.tty,
            cwd,
        });
    }

    sessions.sort_by(|left, right| {
        left.tty
            .cmp(&right.tty)
            .then_with(|| left.pid.cmp(&right.pid))
    });
    Ok(sessions)
}

#[tauri::command]
pub async fn terminate_and_resume_sessions(current_cwd: Option<String>) -> Result<Vec<String>, String> {
    eprintln!("=== terminate_and_resume_sessions 开始 ===");
    eprintln!("当前工作目录: {:?}", current_cwd);

    let all_sessions = find_codex_sessions().await?;
    eprintln!("找到 {} 个 codex 会话", all_sessions.len());

    // 如果指定了工作目录，只处理该目录下的会话
    let sessions: Vec<CodexSession> = if let Some(ref cwd) = current_cwd {
        all_sessions.into_iter()
            .filter(|s| {
                let matches = s.cwd == *cwd;
                if !matches {
                    eprintln!("跳过会话 {} (cwd: {})", s.session_id, s.cwd);
                }
                matches
            })
            .collect()
    } else {
        all_sessions
    };

    eprintln!("过滤后剩余 {} 个会话需要处理", sessions.len());

    let mut resumed = Vec::with_capacity(sessions.len());

    for session in sessions {
        eprintln!("处理会话: session_id={}, pid={}, tty={}, cwd={}",
                  session.session_id, session.pid, session.tty, session.cwd);

        // 检测是否是 VSCode 终端，如果是则跳过
        if is_vscode_terminal(session.pid).await {
            eprintln!("检测到 VSCode 终端，跳过自动恢复: session_id={}", session.session_id);
            continue;
        }

        terminate_process(session.pid).await?;
        eprintln!("已终止进程 {}", session.pid);

        std::thread::sleep(Duration::from_millis(250));

        eprintln!("准备恢复会话 {}", session.session_id);
        match run_terminal_resume_script(&session).await {
            Ok(_) => {
                eprintln!("成功恢复会话 {}", session.session_id);
                resumed.push(session.session_id.clone());
            }
            Err(e) => {
                eprintln!("恢复会话 {} 失败: {}", session.session_id, e);
                return Err(e);
            }
        }
    }

    eprintln!("=== terminate_and_resume_sessions 完成，恢复了 {} 个会话 ===", resumed.len());
    Ok(resumed)
}

#[derive(Debug, Clone)]
struct UsageUpdate {
    auth_json: Value,
    email: Option<String>,
    plan_type: Option<String>,
    usage: Option<crate::models::Usage>,
    usage_error: Option<String>,
}

async fn resolve_usage_update(
    auth_json: &Value,
    fallback_email: Option<String>,
    fallback_plan_type: Option<String>,
) -> UsageUpdate {
    let mut working_auth_json = auth_json.clone();
    let mut extracted = match extract_auth(&working_auth_json) {
        Ok(auth) => auth,
        Err(err) => {
            return UsageUpdate {
                auth_json: working_auth_json,
                email: fallback_email,
                plan_type: fallback_plan_type,
                usage: None,
                usage_error: Some(err),
            };
        }
    };

    let mut refresh_error = None;
    let mut fetch_result = fetch_usage(&extracted.access_token, &extracted.account_id).await;

    if should_retry_with_token_refresh(&fetch_result) {
        match refresh_chatgpt_auth_tokens(&working_auth_json).await {
            Ok(refreshed) => {
                working_auth_json = refreshed;
                match extract_auth(&working_auth_json) {
                    Ok(refreshed_auth) => {
                        extracted = refreshed_auth;
                        fetch_result =
                            fetch_usage(&extracted.access_token, &extracted.account_id).await;
                    }
                    Err(err) => {
                        return UsageUpdate {
                            auth_json: working_auth_json,
                            email: extracted.email.or(fallback_email),
                            plan_type: extracted.plan_type.or(fallback_plan_type),
                            usage: None,
                            usage_error: Some(err),
                        };
                    }
                }
            }
            Err(err) => {
                refresh_error = Some(err);
            }
        }
    }

    match fetch_result {
        Ok(fetched) => UsageUpdate {
            auth_json: working_auth_json,
            email: extracted.email.or(fallback_email),
            plan_type: fetched
                .plan_type
                .or(extracted.plan_type)
                .or(fallback_plan_type),
            usage: Some(fetched.usage),
            usage_error: None,
        },
        Err(err) => UsageUpdate {
            auth_json: working_auth_json,
            email: extracted.email.or(fallback_email),
            plan_type: extracted.plan_type.or(fallback_plan_type),
            usage: None,
            usage_error: Some(match refresh_error {
                Some(refresh_err) => format!("{err} | 令牌刷新失败: {refresh_err}"),
                None => err,
            }),
        },
    }
}

fn should_retry_with_token_refresh(fetch_result: &Result<FetchedUsage, String>) -> bool {
    match fetch_result {
        Ok(snapshot) => snapshot.plan_type.is_none(),
        Err(err) => {
            let normalized = err.to_ascii_lowercase();
            normalized.contains("401")
                || normalized.contains("unauthorized")
                || normalized.contains("invalid_token")
                || normalized.contains("expired")
        }
    }
}

fn current_auth_json_for_account(account_id: &str) -> Option<Value> {
    let auth_json = read_current_codex_auth_optional().ok().flatten()?;
    let extracted = extract_auth(&auth_json).ok()?;
    if extracted.account_id == account_id {
        Some(auth_json)
    } else {
        None
    }
}

fn normalize_account_label(label: &str, fallback: &str) -> String {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn load_accounts_store(app: &AppHandle) -> Result<AccountsStore, String> {
    let path = account_store_path(app)?;
    if !path.exists() {
        return Ok(AccountsStore::default());
    }

    let raw = stdfs::read_to_string(&path)
        .map_err(|e| format!("读取账号存储文件失败 {}: {e}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(AccountsStore::default());
    }

    serde_json::from_str(&raw).map_err(|e| format!("解析账号存储文件失败 {}: {e}", path.display()))
}

fn save_accounts_store(app: &AppHandle, store: &AccountsStore) -> Result<(), String> {
    let path = account_store_path(app)?;
    let parent = path
        .parent()
        .ok_or_else(|| format!("无法解析账号存储目录 {}", path.display()))?;
    stdfs::create_dir_all(parent)
        .map_err(|e| format!("创建账号存储目录失败 {}: {e}", parent.display()))?;

    let serialized =
        serde_json::to_string_pretty(store).map_err(|e| format!("序列化账号存储失败: {e}"))?;
    stdfs::write(&path, serialized)
        .map_err(|e| format!("写入账号存储文件失败 {}: {e}", path.display()))?;
    set_private_permissions(&path);
    Ok(())
}

fn account_store_path(_app: &AppHandle) -> Result<PathBuf, String> {
    let home = env::var("HOME").map_err(|e| format!("无法获取HOME: {e}"))?;
    Ok(PathBuf::from(home)
        .join("Library/Application Support/com.carry.codex-tools")
        .join("accounts.json"))
}

fn app_paths(app: &AppHandle) -> AppResult<AppPaths> {
    if cfg!(debug_assertions) {
        let workspace_root = project_root()?;
        return Ok(AppPaths {
            workspace_root: Some(workspace_root.clone()),
            data_dir: workspace_root.clone(),
            scripts_dir: workspace_root.join("scripts"),
        });
    }

    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|error| AppError::new(format!("无法解析应用资源目录: {error}")))?;
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::new(format!("无法解析应用数据目录: {error}")))?;

    Ok(AppPaths {
        workspace_root: None,
        data_dir,
        scripts_dir: resource_dir.join("scripts"),
    })
}

fn project_root() -> AppResult<PathBuf> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| AppError::new("无法解析项目根目录"))
}

async fn ensure_dir(path: &Path) -> AppResult<()> {
    fs::create_dir_all(path).await?;
    Ok(())
}

async fn ensure_state_file(path: &Path) -> AppResult<()> {
    if fs::try_exists(path).await? {
        return Ok(());
    }

    let state = AccountsState::default();
    save_state(path, &state).await
}

async fn load_state(path: &Path) -> AppResult<AccountsState> {
    ensure_state_file(path).await?;
    let content = fs::read_to_string(path).await?;
    if content.trim().is_empty() {
        return Ok(AccountsState::default());
    }

    Ok(serde_json::from_str(&content)?)
}

async fn save_state(path: &Path, state: &AccountsState) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent).await?;
    }
    let content = serde_json::to_vec_pretty(state)?;
    fs::write(path, content).await?;
    Ok(())
}

fn default_ssh_servers_version() -> u8 {
    1
}

fn ssh_servers_path(paths: &AppPaths) -> PathBuf {
    paths.data_dir.join("servers.json")
}

async fn ensure_ssh_servers_file(path: &Path) -> AppResult<()> {
    if fs::try_exists(path).await? {
        return Ok(());
    }

    let state = SshServersState::default();
    save_ssh_servers_file(path, &state).await
}

async fn load_ssh_servers_state(paths: &AppPaths) -> AppResult<SshServersState> {
    let path = ssh_servers_path(paths);
    ensure_ssh_servers_file(&path).await?;
    let content = fs::read_to_string(&path).await?;
    if content.trim().is_empty() {
        return Ok(SshServersState::default());
    }

    let mut state: SshServersState = serde_json::from_str(&content)?;
    if state.version == 0 {
        state.version = default_ssh_servers_version();
    }
    state.servers = sort_ssh_servers(state.servers);

    Ok(state)
}

async fn save_ssh_servers_state(paths: &AppPaths, state: &SshServersState) -> AppResult<()> {
    let path = ssh_servers_path(paths);
    save_ssh_servers_file(&path, state).await
}

async fn save_ssh_servers_file(path: &Path, state: &SshServersState) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent).await?;
    }

    let content = serde_json::to_vec_pretty(state)?;
    fs::write(path, content).await?;
    Ok(())
}

fn sort_ssh_servers(mut servers: Vec<SshServer>) -> Vec<SshServer> {
    servers.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
            .then_with(|| left.host.cmp(&right.host))
            .then_with(|| left.username.cmp(&right.username))
    });
    servers
}

fn same_ssh_server(left: &SshServer, right: &SshServer) -> bool {
    left.name.eq_ignore_ascii_case(&right.name)
        && left.host.eq_ignore_ascii_case(&right.host)
        && left.port == right.port
        && left.username == right.username
        && left.auth_method == right.auth_method
        && left.key_path == right.key_path
}

fn normalize_ssh_server(server: SshServer) -> Result<SshServer, String> {
    let name = server.name.trim().to_string();
    if name.is_empty() {
        return Err("服务器名称不能为空".to_string());
    }

    let host = server.host.trim().to_string();
    if host.is_empty() {
        return Err("服务器地址不能为空".to_string());
    }

    let username = server.username.trim().to_string();
    if username.is_empty() {
        return Err("用户名不能为空".to_string());
    }

    let auth_method = server.auth_method.trim().to_ascii_lowercase();
    if auth_method != "key" && auth_method != "password" {
        return Err("认证方式必须是 key 或 password".to_string());
    }

    let key_path = if auth_method == "key" {
        let key_path = server.key_path.unwrap_or_default().trim().to_string();
        if key_path.is_empty() {
            return Err("密钥认证需要提供密钥路径".to_string());
        }
        Some(expand_home_path(&key_path))
    } else {
        None
    };

    Ok(SshServer {
        id: server.id.trim().to_string(),
        name,
        host,
        port: if server.port == 0 { 22 } else { server.port },
        username,
        auth_method,
        key_path,
    })
}

fn unique_ssh_server_id(existing: &[SshServer], preferred: Option<&str>) -> String {
    let preferred = preferred.unwrap_or_default().trim();
    if !preferred.is_empty() && existing.iter().all(|server| server.id != preferred) {
        return preferred.to_string();
    }

    loop {
        let candidate = generate_ssh_server_id();
        if existing.iter().all(|server| server.id != candidate) {
            return candidate;
        }
    }
}

fn generate_ssh_server_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("ssh-{nanos:x}")
}

fn parse_ssh_config_content(content: &str) -> Vec<SshServer> {
    let mut servers = Vec::new();
    let mut current = PendingSshHost::default();

    for raw_line in content.lines() {
        let line = raw_line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }

        let mut parts = line.splitn(2, char::is_whitespace);
        let keyword = parts.next().unwrap_or_default().trim().to_ascii_lowercase();
        let value = parts.next().unwrap_or_default().trim();

        match keyword.as_str() {
            "host" => {
                push_pending_ssh_hosts(&mut current, &mut servers);
                current.aliases = value
                    .split_whitespace()
                    .map(|alias| alias.trim().to_string())
                    .filter(|alias| !alias.is_empty())
                    .collect();
            }
            "match" => {
                push_pending_ssh_hosts(&mut current, &mut servers);
            }
            "hostname" if !current.aliases.is_empty() => {
                current.host_name = Some(unquote_ssh_value(value));
            }
            "user" if !current.aliases.is_empty() => {
                current.user = Some(unquote_ssh_value(value));
            }
            "port" if !current.aliases.is_empty() => {
                current.port = value.parse::<u16>().ok();
            }
            "identityfile" if !current.aliases.is_empty() => {
                let identity_file = unquote_ssh_value(value);
                if !identity_file.is_empty() {
                    current.identity_file = Some(expand_home_path(&identity_file));
                }
            }
            _ => {}
        }
    }

    push_pending_ssh_hosts(&mut current, &mut servers);
    dedupe_ssh_servers(sort_ssh_servers(servers))
}

fn push_pending_ssh_hosts(pending: &mut PendingSshHost, servers: &mut Vec<SshServer>) {
    if pending.aliases.is_empty() {
        *pending = PendingSshHost::default();
        return;
    }

    let host_name = pending.host_name.clone();
    let username = pending.user.clone().unwrap_or_else(default_ssh_username);
    let port = pending.port.unwrap_or(22);
    let key_path = pending.identity_file.clone();
    let auth_method = if key_path.is_some() {
        "key"
    } else {
        "password"
    };

    for alias in &pending.aliases {
        if alias.contains('*') || alias.contains('?') || alias.starts_with('!') {
            continue;
        }

        let trimmed_alias = alias.trim();
        if trimmed_alias.is_empty() {
            continue;
        }

        servers.push(SshServer {
            id: String::new(),
            name: trimmed_alias.to_string(),
            host: host_name
                .clone()
                .unwrap_or_else(|| trimmed_alias.to_string()),
            port,
            username: username.clone(),
            auth_method: auth_method.to_string(),
            key_path: key_path.clone(),
        });
    }

    *pending = PendingSshHost::default();
}

fn dedupe_ssh_servers(servers: Vec<SshServer>) -> Vec<SshServer> {
    let mut deduped = Vec::new();

    for server in servers {
        if let Some(existing) = deduped
            .iter_mut()
            .find(|item: &&mut SshServer| item.name.eq_ignore_ascii_case(&server.name))
        {
            *existing = server;
        } else {
            deduped.push(server);
        }
    }

    deduped
}

fn unquote_ssh_value(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn default_ssh_username() -> String {
    env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_default()
}

fn ssh_config_path() -> Result<PathBuf, String> {
    let home_dir = home_dir().ok_or_else(|| "无法解析用户主目录".to_string())?;
    Ok(home_dir.join(".ssh").join("config"))
}

fn codex_auth_path() -> Result<PathBuf, String> {
    let home_dir = home_dir().ok_or_else(|| "无法解析用户主目录".to_string())?;
    Ok(home_dir.join(".codex").join("auth.json"))
}

fn codex_config_path() -> Result<PathBuf, String> {
    let home_dir = home_dir().ok_or_else(|| "无法解析用户主目录".to_string())?;
    Ok(home_dir.join(".codex").join("config.toml"))
}



fn codex_state_db_path() -> Result<PathBuf, String> {
    let codex_dir = home_dir()
        .ok_or_else(|| "无法解析用户主目录".to_string())?
        .join(".codex");
    let entries =
        std::fs::read_dir(&codex_dir).map_err(|error| format!("读取 Codex 目录失败: {error}"))?;
    let mut candidates = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|error| format!("读取 Codex 目录失败: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(name) = path.file_name().and_then(OsStr::to_str) else {
            continue;
        };

        if !name.starts_with("state") || !name.ends_with(".sqlite") {
            continue;
        }

        let modified = entry
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        candidates.push((modified, path));
    }

    candidates
        .into_iter()
        .max_by_key(|(modified, path)| (*modified, path.clone()))
        .map(|(_, path)| path)
        .ok_or_else(|| "未找到 Codex state SQLite 文件".to_string())
}

fn parse_native_codex_resume_process(line: &str) -> Option<ParsedCodexProcess> {
    let mut parts = line.split_whitespace();
    let pid = parts.next()?.parse::<u32>().ok()?;
    let tty = parts.next()?.trim().to_string();
    if tty.is_empty() || tty == "??" {
        return None;
    }

    let command_parts: Vec<&str> = parts.collect();
    if command_parts.is_empty() {
        return None;
    }

    let executable = command_parts.first().copied()?;
    if !is_codex_executable(executable) {
        return None;
    }

    // 如果有子命令，检查是否是我们关心的
    if command_parts.len() >= 2 {
        let subcommand = command_parts.get(1).copied()?;

        // 排除后台服务和非交互式命令
        let excluded_subcommands = ["mcp-server", "app-server", "login", "logout", "version", "help"];
        if excluded_subcommands.contains(&subcommand) {
            return None;
        }

        // 接受 "resume" 和其他交互式命令
    }

    Some(ParsedCodexProcess {
        pid,
        tty,
        command: command_parts.join(" "),
    })
}

fn is_codex_executable(executable: &str) -> bool {
    Path::new(executable)
        .file_name()
        .and_then(OsStr::to_str)
        .map(|name| name == "codex")
        .unwrap_or(false)
}

fn extract_session_from_command(command: &str) -> Option<String> {
    let mut seen_resume = false;

    for token in command.split_whitespace() {
        if seen_resume && looks_like_session_id(token) {
            return Some(token.to_string());
        }
        if token == "resume" {
            seen_resume = true;
        }
    }

    None
}

fn looks_like_session_id(candidate: &str) -> bool {
    const HYPHEN_POSITIONS: [usize; 4] = [8, 13, 18, 23];

    if candidate.len() != 36 {
        return false;
    }

    candidate.chars().enumerate().all(|(index, ch)| {
        if HYPHEN_POSITIONS.contains(&index) {
            ch == '-'
        } else {
            ch.is_ascii_hexdigit()
        }
    })
}

fn query_session_from_sqlite(pid: u32) -> Result<Option<String>, String> {
    let db_path = codex_state_db_path()?;
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("打开 Codex state 数据库失败: {error}"))?;
    let pattern = format!("pid:{pid}:%");

    let session_id: Option<String> = conn
        .query_row(
            "SELECT thread_id
             FROM logs
             WHERE process_uuid LIKE ?1
               AND thread_id IS NOT NULL
               AND thread_id != ''
             ORDER BY id DESC
             LIMIT 1",
            [&pattern],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("查询 Codex session 失败: {error}"))?;

    Ok(session_id.filter(|id| looks_like_session_id(id)))
}

async fn get_process_cwd(pid: u32) -> Result<String, String> {
    let output = Command::new("lsof")
        .arg("-p")
        .arg(pid.to_string())
        .arg("-a")
        .arg("-d")
        .arg("cwd")
        .arg("-Fn")
        .output()
        .await
        .map_err(|error| format!("获取进程工作目录失败: {error}"))?;

    if !output.status.success() {
        return Err("lsof 命令执行失败".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(cwd) = line.strip_prefix('n') {
            return Ok(cwd.to_string());
        }
    }

    Err(format!("无法获取进程 {pid} 的工作目录"))
}

async fn is_vscode_terminal(pid: u32) -> bool {
    eprintln!("=== 检测 VSCode 终端: pid={} ===", pid);

    // 获取进程的完整父进程链
    let mut current_pid = pid;
    let mut depth = 0;
    const MAX_DEPTH: usize = 10;

    while depth < MAX_DEPTH {
        let output = Command::new("ps")
            .arg("-p")
            .arg(current_pid.to_string())
            .arg("-o")
            .arg("ppid=,command=")
            .output()
            .await;

        let Ok(output) = output else {
            eprintln!("无法获取进程 {} 的信息", current_pid);
            return false;
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        eprintln!("进程 {} 信息: {}", current_pid, stdout.trim());

        // 检查当前进程的命令行
        let line_lower = stdout.to_lowercase();
        if line_lower.contains("code") || line_lower.contains("vscode") || line_lower.contains("electron") {
            eprintln!("✓ 检测到 VSCode 相关进程");
            return true;
        }

        // 获取父进程 PID
        let ppid_str = stdout.split_whitespace().next();
        match ppid_str.and_then(|s| s.trim().parse::<u32>().ok()) {
            Some(ppid) if ppid > 1 && ppid != current_pid => {
                current_pid = ppid;
                depth += 1;
            }
            _ => {
                eprintln!("✗ 未检测到 VSCode，已到达进程树顶端");
                return false;
            }
        }
    }

    eprintln!("✗ 未检测到 VSCode，已达到最大深度");
    false
}

async fn terminate_process(pid: u32) -> Result<(), String> {
    let output = Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .output()
        .await
        .map_err(|error| format!("终止 Codex 进程失败: {error}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("No such process") {
        return Ok(());
    }

    Err(format!(
        "终止 Codex 进程 {pid} 失败: {}",
        command_output_summary(&output)
    ))
}

async fn run_terminal_resume_script(session: &CodexSession) -> Result<(), String> {
    // 确保 tty 有 /dev/ 前缀
    let tty = if session.tty.starts_with("/dev/") {
        session.tty.clone()
    } else {
        format!("/dev/{}", session.tty)
    };
    let resume_command = format!("codex resume {}", session.session_id);

    eprintln!("=== run_terminal_resume_script ===");
    eprintln!("tty: {}", tty);
    eprintln!("session_id: {}", session.session_id);
    eprintln!("resume_command: {}", resume_command);

    // 方法1: 尝试 iTerm2 - 在原标签页中执行
    let iterm_script = format!(
        r#"tell application "iTerm2"
    set targetTty to "{tty}"
    set resumeCommand to "{resume_command}"
    set foundSession to false

    repeat with targetWindow in windows
        repeat with targetTab in tabs of targetWindow
            repeat with targetSession in sessions of targetTab
                if tty of targetSession is targetTty then
                    tell targetSession
                        -- 先发送 Ctrl+C 清理终端状态
                        write text (ASCII character 3)
                        delay 0.2
                        -- 然后执行 resume 命令
                        write text resumeCommand
                    end tell
                    activate
                    set foundSession to true
                    exit repeat
                end if
            end repeat
            if foundSession then exit repeat
        end repeat
        if foundSession then exit repeat
    end repeat

    if foundSession then
        return "success"
    else
        error "未找到对应的 iTerm session: {tty}"
    end if
end tell"#
    );

    eprintln!("尝试 iTerm2...");
    let iterm_output = Command::new("osascript")
        .arg("-e")
        .arg(&iterm_script)
        .output()
        .await;

    if let Ok(output) = iterm_output {
        eprintln!("iTerm2 status: {:?}", output.status);
        eprintln!("iTerm2 stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("iTerm2 stderr: {}", String::from_utf8_lossy(&output.stderr));

        if output.status.success() {
            eprintln!("iTerm2 执行成功");
            return Ok(());
        }
    }

    // 方法2: 尝试 Terminal - 也在原标签页中执行
    let terminal_script = format!(
        r#"tell application "Terminal"
    set targetTty to "{tty}"
    set resumeCommand to "{resume_command}"
    set foundTab to false

    repeat with targetWindow in windows
        repeat with targetTab in tabs of targetWindow
            if tty of targetTab is targetTty then
                -- 先发送 Ctrl+C 清理终端状态
                do script (ASCII character 3) in targetTab
                delay 0.2
                -- 然后执行 resume 命令
                do script resumeCommand in targetTab
                activate
                set foundTab to true
                exit repeat
            end if
        end repeat
        if foundTab then exit repeat
    end repeat

    if foundTab then
        return "success"
    else
        error "未找到对应的 Terminal 标签页: {tty}"
    end if
end tell"#
    );

    eprintln!("尝试 Terminal...");
    let output = Command::new("osascript")
        .arg("-e")
        .arg(&terminal_script)
        .output()
        .await
        .map_err(|error| format!("执行 AppleScript 失败: {error}"))?;

    eprintln!("Terminal status: {:?}", output.status);
    eprintln!("Terminal stdout: {}", String::from_utf8_lossy(&output.stdout));
    eprintln!("Terminal stderr: {}", String::from_utf8_lossy(&output.stderr));

    if output.status.success() {
        return Ok(());
    }

    // 方法3: 对于 VSCode 等无法自动恢复的终端，只显示提示信息
    eprintln!("无法在该终端中自动恢复会话");
    eprintln!("请手动在终端中运行: {}", resume_command);

    // 尝试在终端中显示提示信息
    let hint_message = format!(
        "\n=== Codexs 提示 ===\n请运行以下命令恢复会话:\n{}\n==================\n",
        resume_command
    );

    let write_hint = Command::new("sh")
        .arg("-c")
        .arg(format!("printf '%s' '{}' > {}", hint_message, tty))
        .output()
        .await;

    if let Ok(output) = write_hint {
        if output.status.success() {
            eprintln!("已在终端中显示提示信息");
            // 注意：这不算成功恢复，但也不算失败
            // 返回 Ok 以便继续处理其他会话
            return Ok(());
        }
    }

    // 如果连提示信息都无法显示，也返回 Ok，避免中断整个流程
    eprintln!("无法在终端中显示提示信息，用户需要手动恢复会话");
    Ok(())
}

fn expand_home_path(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home_dir) = home_dir() {
            return home_dir.join(stripped).to_string_lossy().into_owned();
        }
    }

    path.to_string()
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
        .or_else(|| {
            let drive = env::var_os("HOMEDRIVE")?;
            let path = env::var_os("HOMEPATH")?;
            let mut home = PathBuf::from(drive);
            home.push(path);
            Some(home)
        })
}

async fn list_json_files(dir: &Path, prefix: Option<&str>) -> AppResult<Vec<PathBuf>> {
    let mut entries = fs::read_dir(dir).await?;
    let mut files = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if !entry.file_type().await?.is_file() {
            continue;
        }
        if path.extension() != Some(OsStr::new("json")) {
            continue;
        }
        if let Some(prefix) = prefix {
            let Some(file_name) = path.file_name().and_then(OsStr::to_str) else {
                continue;
            };
            if !file_name.starts_with(prefix) {
                continue;
            }
        }
        files.push(path);
    }

    Ok(files)
}

async fn detect_newest_file(
    before_files: Vec<PathBuf>,
    after_files: Vec<PathBuf>,
) -> AppResult<Option<PathBuf>> {
    let before_names: HashSet<String> = before_files
        .iter()
        .filter_map(|path| path.file_name().and_then(OsStr::to_str))
        .map(ToOwned::to_owned)
        .collect();

    let candidates: Vec<PathBuf> = after_files
        .into_iter()
        .filter(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .map(|name| !before_names.contains(name))
                .unwrap_or(false)
        })
        .collect();

    newest_file(candidates).await
}

async fn newest_file(files: Vec<PathBuf>) -> AppResult<Option<PathBuf>> {
    let mut latest: Option<(PathBuf, SystemTime)> = None;

    for path in files {
        let modified = fs::metadata(&path)
            .await?
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH);

        match &latest {
            Some((_, current)) if modified < *current => {}
            _ => latest = Some((path, modified)),
        }
    }

    Ok(latest.map(|(path, _)| path))
}

async fn run_python_script(
    paths: &AppPaths,
    current_dir: &Path,
    script_name: &str,
    args: &[&str],
) -> AppResult<Output> {
    // For openai_register.py, use the bundled binary if available
    if script_name == "openai_register.py" {
        let binary_name = "openai_register_bin";
        let binary_path = paths.scripts_dir.join(binary_name);

        if binary_path.exists() {
            let mut command = Command::new(&binary_path);
            command.current_dir(current_dir).args(args);

            let output = command.output().await.map_err(|error| {
                AppError::new(format!(
                    "执行独立可执行文件失败 ({}): {error}",
                    binary_path.display()
                ))
            })?;

            return Ok(output);
        }
    }

    // Fallback to Python script
    let script_path = paths.scripts_dir.join(script_name);
    if !script_path.exists() {
        return Err(AppError::new(format!(
            "Python 脚本不存在: {}",
            script_path.display()
        )));
    }

    let python = python_executable(paths);
    let mut command = Command::new(&python);
    command
        .current_dir(current_dir)
        .arg(&script_path)
        .args(args);

    let output = command.output().await.map_err(|error| {
        AppError::new(format!(
            "执行 Python 脚本失败 ({}, python={}): {error}",
            script_path.display(),
            python.display()
        ))
    })?;

    Ok(output)
}

async fn convert_generated_token(
    root: &Path,
    raw_token_path: &Path,
    codex_tokens_dir: &Path,
) -> AppResult<GeneratedAccount> {
    let raw_content = fs::read_to_string(raw_token_path).await?;
    let raw_token: RawGeneratedToken = serde_json::from_str(&raw_content)?;

    let email = if raw_token.email.trim().is_empty() {
        extract_email_from_id_token(&raw_token.id_token)?
    } else {
        normalize_email(&raw_token.email)
    };

    if email.is_empty() {
        return Err(AppError::new(format!(
            "无法从 {} 解析邮箱",
            raw_token_path.display()
        )));
    }

    if raw_token.id_token.trim().is_empty() {
        return Err(AppError::new(format!(
            "{} 缺少 id_token",
            raw_token_path.display()
        )));
    }

    let created_at = if raw_token.last_refresh.trim().is_empty() {
        now_rfc3339()
    } else {
        raw_token.last_refresh
    };

    let codex_token_path =
        codex_tokens_dir.join(format!("{}.json", sanitize_email_for_filename(&email)));
    let converted = json!({
        "OPENAI_API_KEY": Value::Null,
        "auth_mode": "chatgpt",
        "last_refresh": created_at,
        "tokens": {
            "access_token": raw_token.access_token,
            "account_id": raw_token.account_id,
            "id_token": raw_token.id_token,
            "refresh_token": raw_token.refresh_token
        }
    });

    fs::write(&codex_token_path, serde_json::to_vec_pretty(&converted)?).await?;

    Ok(GeneratedAccount {
        email,
        created_at: converted["last_refresh"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        token_path: relative_path_string(root, raw_token_path),
        codex_token_path: relative_path_string(root, &codex_token_path),
    })
}

async fn read_codex_accounts(paths: &AppPaths) -> AppResult<Vec<DiscoveredCodexAccount>> {
    let codex_tokens_dir = paths.data_dir.join("codex_tokens");
    ensure_dir(&codex_tokens_dir).await?;

    let files = list_json_files(&codex_tokens_dir, None).await?;
    let mut accounts = Vec::new();

    for path in files {
        match read_single_codex_account(&path).await {
            Ok(Some(account)) => accounts.push(account),
            Ok(None) => {}
            Err(error) => warn!("跳过损坏的 token 文件 {}: {}", path.display(), error),
        }
    }

    Ok(accounts)
}

async fn read_single_codex_account(path: &Path) -> AppResult<Option<DiscoveredCodexAccount>> {
    let content = fs::read_to_string(path).await?;
    let token_file: CodexTokenFile = serde_json::from_str(&content)?;

    let email = extract_email_from_id_token(&token_file.tokens.id_token)?;
    if email.is_empty() {
        return Ok(None);
    }

    let created_at = if token_file.last_refresh.trim().is_empty() {
        let modified = fs::metadata(path)
            .await?
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH);
        system_time_to_rfc3339(modified)
    } else {
        token_file.last_refresh
    };

    Ok(Some(DiscoveredCodexAccount { email, created_at }))
}

fn extract_email_from_id_token(id_token: &str) -> AppResult<String> {
    let payload = id_token
        .split('.')
        .nth(1)
        .ok_or_else(|| AppError::new("id_token 格式不合法"))?;

    let claims = decode_jwt_payload(payload)?;
    let email = claims
        .get("email")
        .and_then(Value::as_str)
        .map(normalize_email)
        .unwrap_or_default();

    Ok(email)
}

fn decode_jwt_payload(payload: &str) -> AppResult<Value> {
    let mut padded = payload.to_string();
    while padded.len() % 4 != 0 {
        padded.push('=');
    }

    let decoded = URL_SAFE
        .decode(padded.as_bytes())
        .map_err(|error| AppError::new(format!("JWT payload 解码失败: {error}")))?;

    Ok(serde_json::from_slice(&decoded)?)
}

fn mark_accounts_imported(state: &mut AccountsState, imported_emails: &[String]) {
    let imported_lookup: HashMap<String, usize> = state
        .accounts
        .iter()
        .enumerate()
        .map(|(index, entry)| (normalize_email(&entry.email), index))
        .collect();

    let imported_at = now_rfc3339();

    for email in imported_emails {
        let normalized = normalize_email(email);
        if let Some(index) = imported_lookup.get(&normalized).copied() {
            if let Some(entry) = state.accounts.get_mut(index) {
                entry.imported = true;
                entry.imported_at = Some(imported_at.clone());
            }
            continue;
        }

        state.accounts.push(AccountStateEntry {
            email: normalized,
            imported: true,
            imported_at: Some(imported_at.clone()),
        });
    }

    state
        .accounts
        .sort_by(|left, right| left.email.cmp(&right.email));
}

fn track_generated_account(state: &mut AccountsState, email: &str) -> bool {
    let normalized = normalize_email(email);
    if let Some(entry) = state
        .accounts
        .iter_mut()
        .find(|entry| normalize_email(&entry.email) == normalized)
    {
        entry.email = normalized;
        return entry.imported;
    }

    state.accounts.push(AccountStateEntry {
        email: normalized,
        imported: false,
        imported_at: None,
    });
    state
        .accounts
        .sort_by(|left, right| left.email.cmp(&right.email));

    false
}

fn emit_progress(
    app: &AppHandle,
    current: u32,
    total: u32,
    email: String,
    account: Option<AvailableAccount>,
) {
    if let Err(error) = app.emit(
        GENERATION_PROGRESS_EVENT,
        GenerationProgressEvent {
            current,
            total,
            email,
            account,
        },
    ) {
        warn!("发送生成进度事件失败: {error}");
    }
}

fn parse_import_summary(stdout: &str) -> Option<ImportScriptSummary> {
    stdout
        .lines()
        .rev()
        .find_map(|line| line.strip_prefix("SUMMARY_JSON:"))
        .and_then(|payload| serde_json::from_str(payload).ok())
}

fn command_output_summary(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let mut fragments = Vec::new();

    if let Some(code) = output.status.code() {
        fragments.push(format!("exit code {code}"));
    } else {
        fragments.push("exit code unavailable".to_string());
    }

    if !stdout.is_empty() {
        fragments.push(format!("stdout: {stdout}"));
    }

    if !stderr.is_empty() {
        fragments.push(format!("stderr: {stderr}"));
    }

    fragments.join(" | ")
}

fn sanitize_email_for_filename(email: &str) -> String {
    email.replace('@', "_").replace('.', "_")
}

fn normalize_email(email: &str) -> String {
    email.trim().to_ascii_lowercase()
}

fn unique_emails(emails: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();

    for email in emails {
        let normalized = normalize_email(&email);
        if normalized.is_empty() || !seen.insert(normalized.clone()) {
            continue;
        }
        unique.push(normalized);
    }

    unique
}

fn relative_path_string(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|relative| relative.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string_lossy().into_owned())
}

fn python_executable(paths: &AppPaths) -> PathBuf {
    if let Some(workspace_root) = &paths.workspace_root {
        let venv_python = workspace_root.join(".venv").join("bin").join("python3");
        if venv_python.exists() {
            return venv_python;
        }
    }

    for candidate in [
        "/opt/homebrew/bin/python3",
        "/usr/local/bin/python3",
        "/usr/bin/python3",
    ] {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return path;
        }
    }

    PathBuf::from("python3")
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn system_time_to_rfc3339(time: SystemTime) -> String {
    DateTime::<Utc>::from(time).to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_ssh_config_entries() {
        let config = r#"
            Host web-prod
              HostName 10.0.0.8
              User ubuntu
              Port 2222
              IdentityFile ~/.ssh/id_ed25519

            Host db-prod
              HostName db.internal
              User admin
        "#;

        let servers = parse_ssh_config_content(config);
        assert_eq!(servers.len(), 2);

        let db = servers
            .iter()
            .find(|server| server.name == "db-prod")
            .unwrap();
        assert_eq!(db.host, "db.internal");
        assert_eq!(db.username, "admin");
        assert_eq!(db.port, 22);
        assert_eq!(db.auth_method, "password");
        assert_eq!(db.key_path, None);

        let web = servers
            .iter()
            .find(|server| server.name == "web-prod")
            .unwrap();
        assert_eq!(web.host, "10.0.0.8");
        assert_eq!(web.username, "ubuntu");
        assert_eq!(web.port, 2222);
        assert_eq!(web.auth_method, "key");
        assert!(web
            .key_path
            .as_deref()
            .unwrap_or_default()
            .ends_with(".ssh/id_ed25519"));
    }

    #[test]
    fn skips_wildcards_and_expands_multi_host_entries() {
        let config = r#"
            Host *.internal
              User ignored

            Host api-stage api-canary
              HostName 172.16.0.20
              User deploy
              Port 2200
        "#;

        let servers = parse_ssh_config_content(config);
        assert_eq!(servers.len(), 2);
        assert!(servers.iter().all(|server| server.name != "*.internal"));
        assert!(servers.iter().any(|server| server.name == "api-stage"));
        assert!(servers.iter().any(|server| server.name == "api-canary"));
        assert!(servers.iter().all(|server| server.host == "172.16.0.20"));
        assert!(servers.iter().all(|server| server.username == "deploy"));
        assert!(servers.iter().all(|server| server.port == 2200));
    }

    #[test]
    fn extracts_session_id_from_resume_command() {
        let command = "/opt/homebrew/lib/node_modules/@openai/codex/node_modules/@openai/codex-darwin-arm64/vendor/aarch64-apple-darwin/codex/codex resume 019ccb6b-a99e-7293-8c80-a0613c5bc3ce";
        let session_id = extract_session_from_command(command);
        assert_eq!(
            session_id.as_deref(),
            Some("019ccb6b-a99e-7293-8c80-a0613c5bc3ce")
        );
    }

    #[test]
    fn parses_native_codex_resume_process_line() {
        let line = "51943 ttys018 /opt/homebrew/lib/node_modules/@openai/codex/node_modules/@openai/codex-darwin-arm64/vendor/aarch64-apple-darwin/codex/codex resume";
        let process = parse_native_codex_resume_process(line).unwrap();
        assert_eq!(process.pid, 51943);
        assert_eq!(process.tty, "ttys018");
        assert_eq!(
            process.command,
            "/opt/homebrew/lib/node_modules/@openai/codex/node_modules/@openai/codex-darwin-arm64/vendor/aarch64-apple-darwin/codex/codex resume"
        );
    }

    #[test]
    fn ignores_non_resume_codex_processes() {
        assert!(parse_native_codex_resume_process(
            "59385 ttys003 /opt/homebrew/lib/node_modules/@openai/codex/node_modules/@openai/codex-darwin-arm64/vendor/aarch64-apple-darwin/codex/codex mcp-server"
        )
        .is_none());
        assert!(parse_native_codex_resume_process(
            "51941 ttys018 node /opt/homebrew/bin/codex resume 019ccb6b-a99e-7293-8c80-a0613c5bc3ce"
        )
        .is_none());
    }
}
