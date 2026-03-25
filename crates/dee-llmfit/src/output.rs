use serde::Serialize;

#[derive(Debug, Clone, Copy)]
pub struct OutputMode {
    pub json: bool,
    pub quiet: bool,
    pub verbose: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Ambiguous(String),
    #[error("{0}")]
    Internal(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidArgument(_) => "INVALID_ARGUMENT",
            Self::NotFound(_) => "NOT_FOUND",
            Self::Ambiguous(_) => "AMBIGUOUS",
            Self::Internal(_) => "INTERNAL",
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(value: anyhow::Error) -> Self {
        Self::Internal(value.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[derive(Serialize)]
struct ErrorJson<'a> {
    ok: bool,
    error: &'a str,
    code: &'a str,
}

#[derive(Serialize)]
pub struct ListJson<T: Serialize> {
    pub ok: bool,
    pub count: usize,
    pub items: Vec<T>,
}

#[derive(Serialize)]
pub struct ItemJson<T: Serialize> {
    pub ok: bool,
    pub item: T,
}

pub fn print_json<T: Serialize>(value: &T) -> AppResult<()> {
    let out = serde_json::to_string(value).map_err(|e| AppError::Internal(e.to_string()))?;
    println!("{out}");
    Ok(())
}

pub fn print_error(err: &AppError, json: bool) {
    if json {
        let payload = ErrorJson {
            ok: false,
            error: &err.to_string(),
            code: err.code(),
        };
        match serde_json::to_string(&payload) {
            Ok(text) => println!("{text}"),
            Err(_) => println!(
                "{{\"ok\":false,\"error\":\"Internal serialization error\",\"code\":\"SERIALIZE\"}}"
            ),
        }
    } else {
        eprintln!("error: {err}");
    }
}
