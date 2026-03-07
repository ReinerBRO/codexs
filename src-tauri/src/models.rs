use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResult {
    pub requested: u32,
    pub succeeded: u32,
    pub failed: u32,
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
