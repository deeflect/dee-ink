use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Data directory not found")]
    DataDirMissing,
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Not found")]
    NotFound,
    #[error("Authentication missing for platform: {0}")]
    AuthMissing(String),
    #[error("Request failed: {0}")]
    RequestFailed(String),
    #[error("Database operation failed")]
    Database,
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::DataDirMissing => "CONFIG_MISSING",
            Self::InvalidArgument(_) => "INVALID_ARGUMENT",
            Self::NotFound => "NOT_FOUND",
            Self::AuthMissing(_) => "AUTH_MISSING",
            Self::RequestFailed(_) => "REQUEST_FAILED",
            Self::Database => "DATABASE_ERROR",
        }
    }
}
