use crate::models::{
    Account, AccountStateEntry, AccountsState, AppError, AppResult, GeneratedAccount,
    GenerationProgressEvent, GenerationResult, ImportResult,
};
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use chrono::{DateTime, SecondsFormat, Utc};
use log::warn;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Output,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::SystemTime,
};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::{fs, process::Command};

const GENERATION_PROGRESS_EVENT: &str = "generation_progress";

#[derive(Clone, Default)]
pub struct GenerationControl {
    stop_requested: Arc<AtomicBool>,
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
struct StoredAccount {
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

                let emitted_account = Account {
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
pub async fn get_accounts(app: AppHandle) -> AppResult<Vec<Account>> {
    let paths = app_paths(&app)?;
    let state_path = paths.data_dir.join("accounts_state.json");
    ensure_state_file(&state_path).await?;

    let state = load_state(&state_path).await?;
    let imported_lookup: HashMap<String, bool> = state
        .accounts
        .into_iter()
        .map(|entry| (normalize_email(&entry.email), entry.imported))
        .collect();

    let mut accounts: Vec<Account> = read_codex_accounts(&paths)
        .await?
        .into_iter()
        .map(|account| Account {
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

async fn read_codex_accounts(paths: &AppPaths) -> AppResult<Vec<StoredAccount>> {
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

async fn read_single_codex_account(path: &Path) -> AppResult<Option<StoredAccount>> {
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

    Ok(Some(StoredAccount { email, created_at }))
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
    account: Option<Account>,
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
