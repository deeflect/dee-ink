use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub ok: bool,
    pub error: String,
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct ActionResponse {
    pub ok: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListResponse<T> {
    pub ok: bool,
    pub count: usize,
    pub items: Vec<T>,
}

#[derive(Debug, Serialize)]
pub struct ItemResponse<T> {
    pub ok: bool,
    pub item: T,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthStatusItem {
    pub platform: String,
    pub configured: bool,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostPlatformResult {
    pub platform: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_post_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueueItem {
    pub id: String,
    pub run_at: String,
    pub status: String,
    pub text: String,
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueueDetail {
    pub id: String,
    pub run_at: String,
    pub status: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subreddit: Option<String>,
    pub targets: Vec<PostPlatformResult>,
}

#[derive(Debug, Serialize)]
pub struct PostResponse {
    pub ok: bool,
    pub count: usize,
    pub results: Vec<PostPlatformResult>,
}

#[derive(Debug, Serialize)]
pub struct RunResponse {
    pub ok: bool,
    pub jobs_processed: usize,
    pub targets_sent: usize,
    pub targets_failed: usize,
}
