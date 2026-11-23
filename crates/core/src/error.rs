use thiserror::Error;

/// Errors produced by the domain layer.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("قيمة غير صالحة: {0}")]
    Invalid(String),
    #[error("خطأ في الإدخال/الإخراج: {0}")]
    Io(#[from] std::io::Error),
    #[error("خطأ في التحويل: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
