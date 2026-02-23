use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub struct OutputMode {
    pub json: bool,
    pub quiet: bool,
    pub verbose: bool,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Wikipedia request failed")]
    Request,
    #[error("Wikipedia response could not be parsed")]
    Parse,
    #[error("No article found")]
    NotFound,
    #[error("Invalid language code")]
    InvalidLanguage,
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Request => "REQUEST_FAILED",
            Self::Parse => "PARSE_FAILED",
            Self::NotFound => "NOT_FOUND",
            Self::InvalidLanguage => "INVALID_LANGUAGE",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorJson {
    pub ok: bool,
    pub error: String,
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct SearchItem {
    pub title: String,
    pub description: String,
    pub url: String,
    pub lang: String,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub ok: bool,
    pub count: usize,
    pub items: Vec<SearchItem>,
}

#[derive(Debug, Serialize)]
pub struct WikiItem {
    pub title: String,
    pub extract: String,
    pub url: String,
    pub thumbnail: String,
    pub lang: String,
}

#[derive(Debug, Serialize)]
pub struct ItemResponse {
    pub ok: bool,
    pub item: WikiItem,
}

#[derive(Debug, serde::Deserialize)]
pub struct SummaryApi {
    pub title: Option<String>,
    pub extract: Option<String>,
    pub content_urls: Option<ContentUrls>,
    pub thumbnail: Option<Thumbnail>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ContentUrls {
    pub desktop: Option<Desktop>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Desktop {
    pub page: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Thumbnail {
    pub source: Option<String>,
}
