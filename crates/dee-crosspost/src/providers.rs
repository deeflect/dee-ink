use std::env;

use reqwest::Client;
use serde::Deserialize;

use crate::cli::Platform;
use crate::error::AppError;
use crate::models::PostPlatformResult;

#[derive(Debug, Clone)]
pub struct PostRequest {
    pub text: String,
    pub media: Option<String>,
    pub title: Option<String>,
    pub subreddit: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ThreadsCreateResponse {
    #[serde(default)]
    id: String,
}

#[derive(Debug, Deserialize)]
struct ThreadsPublishResponse {
    #[serde(default)]
    id: String,
}

#[derive(Debug, Deserialize)]
struct BlueskyResponse {
    #[serde(default)]
    uri: Option<String>,
    #[serde(default)]
    cid: Option<String>,
}

#[derive(Debug, Deserialize)]
struct XPostResponse {
    data: XPostData,
}

#[derive(Debug, Deserialize)]
struct XPostData {
    id: String,
}

#[derive(Debug, Deserialize)]
struct RedditResponse {
    json: RedditJson,
}

#[derive(Debug, Deserialize)]
struct RedditJson {
    errors: Vec<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct LinkedInResponse {
    id: String,
}

pub async fn post_to_platform(
    client: &Client,
    platform: Platform,
    token: &str,
    req: &PostRequest,
) -> Result<PostPlatformResult, AppError> {
    match platform {
        Platform::X => post_x(client, token, req).await,
        Platform::Linkedin => post_linkedin(client, token, req).await,
        Platform::Bluesky => post_bluesky(client, token, req).await,
        Platform::Threads => post_threads(client, token, req).await,
        Platform::Reddit => post_reddit(client, token, req).await,
    }
}

fn validate_common(req: &PostRequest) -> Result<(), AppError> {
    if req.text.trim().is_empty() {
        return Err(AppError::InvalidArgument("text cannot be empty".into()));
    }
    if req.text.chars().count() > 5000 {
        return Err(AppError::InvalidArgument(
            "text exceeds 5000 characters".into(),
        ));
    }
    if let Some(path) = &req.media {
        if path.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                "media path cannot be empty".into(),
            ));
        }
    }
    Ok(())
}

async fn post_x(
    client: &Client,
    token: &str,
    req: &PostRequest,
) -> Result<PostPlatformResult, AppError> {
    validate_common(req)?;
    if req.text.chars().count() > 280 {
        return Err(AppError::InvalidArgument(
            "x text exceeds 280 characters".into(),
        ));
    }

    let base = env::var("DEE_CROSSPOST_X_BASE").unwrap_or_else(|_| "https://api.x.com".to_string());
    let response = client
        .post(format!("{base}/2/tweets"))
        .bearer_auth(token)
        .json(&serde_json::json!({ "text": req.text }))
        .send()
        .await
        .map_err(|e| AppError::RequestFailed(format!("x: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::RequestFailed(format!("x http {status}: {body}")));
    }

    let payload: XPostResponse = response
        .json()
        .await
        .map_err(|e| AppError::RequestFailed(format!("x json: {e}")))?;

    Ok(PostPlatformResult {
        platform: "x".to_string(),
        status: "sent".to_string(),
        remote_post_id: Some(payload.data.id.clone()),
        url: Some(format!("https://x.com/i/web/status/{}", payload.data.id)),
        error: None,
    })
}

async fn post_linkedin(
    client: &Client,
    token: &str,
    req: &PostRequest,
) -> Result<PostPlatformResult, AppError> {
    validate_common(req)?;
    let actor = env::var("DEE_CROSSPOST_LINKEDIN_ACTOR").map_err(|_| {
        AppError::InvalidArgument(
            "linkedin requires DEE_CROSSPOST_LINKEDIN_ACTOR (e.g. urn:li:person:...)".into(),
        )
    })?;
    let base = env::var("DEE_CROSSPOST_LINKEDIN_BASE")
        .unwrap_or_else(|_| "https://api.linkedin.com".to_string());

    let response = client
        .post(format!("{base}/v2/ugcPosts"))
        .bearer_auth(token)
        .header("X-Restli-Protocol-Version", "2.0.0")
        .json(&serde_json::json!({
            "author": actor,
            "lifecycleState": "PUBLISHED",
            "specificContent": {
                "com.linkedin.ugc.ShareContent": {
                    "shareCommentary": { "text": req.text },
                    "shareMediaCategory": "NONE"
                }
            },
            "visibility": {
                "com.linkedin.ugc.MemberNetworkVisibility": "PUBLIC"
            }
        }))
        .send()
        .await
        .map_err(|e| AppError::RequestFailed(format!("linkedin: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::RequestFailed(format!(
            "linkedin http {status}: {body}"
        )));
    }

    let payload: LinkedInResponse = response
        .json()
        .await
        .map_err(|e| AppError::RequestFailed(format!("linkedin json: {e}")))?;

    Ok(PostPlatformResult {
        platform: "linkedin".to_string(),
        status: "sent".to_string(),
        remote_post_id: Some(payload.id),
        url: None,
        error: None,
    })
}

async fn post_bluesky(
    client: &Client,
    token: &str,
    req: &PostRequest,
) -> Result<PostPlatformResult, AppError> {
    validate_common(req)?;
    let repo = env::var("DEE_CROSSPOST_BLUESKY_REPO").map_err(|_| {
        AppError::InvalidArgument("bluesky requires DEE_CROSSPOST_BLUESKY_REPO".into())
    })?;
    let base = env::var("DEE_CROSSPOST_BLUESKY_BASE")
        .unwrap_or_else(|_| "https://bsky.social".to_string());

    let response = client
        .post(format!("{base}/xrpc/com.atproto.repo.createRecord"))
        .bearer_auth(token)
        .json(&serde_json::json!({
            "repo": repo,
            "collection": "app.bsky.feed.post",
            "record": {
                "$type": "app.bsky.feed.post",
                "text": req.text,
                "createdAt": chrono::Utc::now().to_rfc3339()
            }
        }))
        .send()
        .await
        .map_err(|e| AppError::RequestFailed(format!("bluesky: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::RequestFailed(format!(
            "bluesky http {status}: {body}"
        )));
    }

    let payload: BlueskyResponse = response
        .json()
        .await
        .map_err(|e| AppError::RequestFailed(format!("bluesky json: {e}")))?;

    Ok(PostPlatformResult {
        platform: "bluesky".to_string(),
        status: "sent".to_string(),
        remote_post_id: payload.cid,
        url: payload.uri,
        error: None,
    })
}

async fn post_threads(
    client: &Client,
    token: &str,
    req: &PostRequest,
) -> Result<PostPlatformResult, AppError> {
    validate_common(req)?;
    let user_id = env::var("DEE_CROSSPOST_THREADS_USER_ID").map_err(|_| {
        AppError::InvalidArgument("threads requires DEE_CROSSPOST_THREADS_USER_ID".into())
    })?;
    let base = env::var("DEE_CROSSPOST_THREADS_BASE")
        .unwrap_or_else(|_| "https://graph.threads.net".to_string());

    let create = client
        .post(format!("{base}/v1.0/{user_id}/threads"))
        .bearer_auth(token)
        .form(&[("text", req.text.as_str()), ("media_type", "TEXT")])
        .send()
        .await
        .map_err(|e| AppError::RequestFailed(format!("threads create: {e}")))?;

    if !create.status().is_success() {
        let status = create.status();
        let body = create.text().await.unwrap_or_default();
        return Err(AppError::RequestFailed(format!(
            "threads create http {status}: {body}"
        )));
    }

    let created: ThreadsCreateResponse = create
        .json()
        .await
        .map_err(|e| AppError::RequestFailed(format!("threads create json: {e}")))?;

    let publish = client
        .post(format!("{base}/v1.0/{user_id}/threads_publish"))
        .bearer_auth(token)
        .form(&[("creation_id", created.id.as_str())])
        .send()
        .await
        .map_err(|e| AppError::RequestFailed(format!("threads publish: {e}")))?;

    if !publish.status().is_success() {
        let status = publish.status();
        let body = publish.text().await.unwrap_or_default();
        return Err(AppError::RequestFailed(format!(
            "threads publish http {status}: {body}"
        )));
    }

    let payload: ThreadsPublishResponse = publish
        .json()
        .await
        .map_err(|e| AppError::RequestFailed(format!("threads publish json: {e}")))?;

    Ok(PostPlatformResult {
        platform: "threads".to_string(),
        status: "sent".to_string(),
        remote_post_id: Some(payload.id),
        url: None,
        error: None,
    })
}

async fn post_reddit(
    client: &Client,
    token: &str,
    req: &PostRequest,
) -> Result<PostPlatformResult, AppError> {
    validate_common(req)?;
    let subreddit = req
        .subreddit
        .clone()
        .ok_or_else(|| AppError::InvalidArgument("reddit requires --subreddit".into()))?;
    let title = req
        .title
        .clone()
        .ok_or_else(|| AppError::InvalidArgument("reddit requires --title".into()))?;

    let base = env::var("DEE_CROSSPOST_REDDIT_BASE")
        .unwrap_or_else(|_| "https://oauth.reddit.com".to_string());

    let response = client
        .post(format!("{base}/api/submit"))
        .bearer_auth(token)
        .header("User-Agent", "dee-crosspost/0.1")
        .form(&[
            ("sr", subreddit.as_str()),
            ("kind", "self"),
            ("title", title.as_str()),
            ("text", req.text.as_str()),
            ("api_type", "json"),
        ])
        .send()
        .await
        .map_err(|e| AppError::RequestFailed(format!("reddit: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::RequestFailed(format!(
            "reddit http {status}: {body}"
        )));
    }

    let payload: RedditResponse = response
        .json()
        .await
        .map_err(|e| AppError::RequestFailed(format!("reddit json: {e}")))?;

    if !payload.json.errors.is_empty() {
        return Err(AppError::RequestFailed(format!(
            "reddit api errors: {:?}",
            payload.json.errors
        )));
    }

    Ok(PostPlatformResult {
        platform: "reddit".to_string(),
        status: "sent".to_string(),
        remote_post_id: None,
        url: None,
        error: None,
    })
}
