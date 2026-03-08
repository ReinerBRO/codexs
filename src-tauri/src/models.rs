use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableAccount {
    pub email: String,
    pub created_at: String,
    pub imported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedAccount {
    pub email: String,
    pub created_at: String,
    pub token_path: String,
    pub codex_token_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationProgressEvent {
    pub current: u32,
    pub total: u32,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<AvailableAccount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResult {
    pub requested: u32,
    pub succeeded: u32,
    pub failed: u32,
    pub stopped: bool,
    pub accounts: Vec<GeneratedAccount>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub requested: usize,
    pub imported: usize,
    pub skipped: usize,
    pub failed: usize,
    pub emails: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccountsState {
    #[serde(default)]
    pub accounts: Vec<AccountStateEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountStateEntry {
    pub email: String,
    pub imported: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imported_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub label: String,
    pub email: Option<String>,
    pub account_id: String,
    pub plan_type: Option<String>,
    pub auth_json: Value,
    pub added_at: i64,
    pub updated_at: i64,
    pub usage: Option<Usage>,
    pub usage_error: Option<String>,
    pub is_current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    #[serde(alias = "fiveHour")]
    pub five_hour: UsageWindow,
    #[serde(alias = "oneWeek")]
    pub one_week: UsageWindow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageWindow {
    #[serde(alias = "usedPercent")]
    pub used_percent: f64,
    #[serde(alias = "windowSeconds", default)]
    pub window_seconds: i64,
    #[serde(alias = "resetAt", default)]
    pub reset_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredAccount {
    pub id: String,
    pub label: String,
    pub email: Option<String>,
    #[serde(alias = "accountId")]
    pub account_id: String,
    #[serde(alias = "planType")]
    pub plan_type: Option<String>,
    #[serde(alias = "authJson")]
    pub auth_json: Value,
    #[serde(alias = "addedAt")]
    pub added_at: i64,
    #[serde(alias = "updatedAt")]
    pub updated_at: i64,
    #[serde(default)]
    pub usage: Option<Usage>,
    #[serde(alias = "usageError")]
    pub usage_error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccountsStore {
    #[serde(default)]
    pub accounts: Vec<StoredAccount>,
    #[serde(default)]
    pub settings: StoreSettings,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoreSettings {}

#[derive(Debug, Clone)]
pub struct ExtractedAuth {
    pub account_id: String,
    pub access_token: String,
    pub email: Option<String>,
    pub plan_type: Option<String>,
}

impl StoredAccount {
    pub fn to_account(&self, current_account_id: Option<&str>) -> Account {
        Account {
            id: self.id.clone(),
            label: self.label.clone(),
            email: self.email.clone(),
            account_id: self.account_id.clone(),
            plan_type: self.plan_type.clone(),
            auth_json: self.auth_json.clone(),
            added_at: self.added_at,
            updated_at: self.updated_at,
            usage: self.usage.clone(),
            usage_error: self.usage_error.clone(),
            is_current: current_account_id
                .map(|id| id == self.account_id)
                .unwrap_or(false),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppError {
    message: String,
}

impl AppError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for AppError {}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.message)
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::new(format!("I/O error: {value}"))
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        Self::new(format!("JSON error: {value}"))
    }
}

pub type AppResult<T> = Result<T, AppError>;
