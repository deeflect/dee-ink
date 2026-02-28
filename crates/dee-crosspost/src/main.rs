mod cli;
mod db;
mod error;
mod models;
mod providers;

use std::env;
use std::time::Duration;

use chrono::{DateTime, Utc};
use clap::{CommandFactory, Parser};
use cli::{
    AuthArgs, AuthCommand, Cli, Commands, Platform, PostArgs, QueueArgs, QueueCommand, RunArgs,
    ScheduleArgs,
};
use db::{
    auth_status_db_map, connect, db_path, delete_token, due_jobs, finalize_job, get_token,
    get_token_expiry, mark_target_result, queue_cancel, queue_list, queue_show, schedule_job,
    set_job_running, upsert_token, PostDraft,
};
use error::AppError;
use models::{
    ActionResponse, AuthStatusItem, ErrorResponse, ItemResponse, ListResponse, PostPlatformResult,
    PostResponse, RunResponse,
};
use providers::{post_to_platform, PostRequest};
use reqwest::Client;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(err) = run(cli).await {
        print_error(&err);
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<(), AppError> {
    let path = db_path()?;
    let conn = connect(&path)?;

    match cli.command {
        Commands::Auth(args) => handle_auth(&conn, args, cli.global.json),
        Commands::Post(args) => handle_post(&conn, args, cli.global.json).await,
        Commands::Schedule(args) => handle_schedule(&conn, args, cli.global.json),
        Commands::Queue(args) => handle_queue(&conn, args, cli.global.json),
        Commands::Run(args) => handle_run(&conn, args, cli.global.json).await,
    }
}

fn handle_auth(conn: &rusqlite::Connection, args: AuthArgs, json: bool) -> Result<(), AppError> {
    match args.command {
        AuthCommand::SetToken(args) => {
            if args.token.trim().is_empty() {
                return Err(AppError::InvalidArgument("token cannot be empty".into()));
            }
            upsert_token(conn, args.platform, &args.token)?;
            print_json_or_text(
                json,
                &ActionResponse {
                    ok: true,
                    message: format!("token saved for {}", args.platform.as_str()),
                    id: None,
                },
                "token saved",
            );
            Ok(())
        }
        AuthCommand::Status => {
            let db_map = auth_status_db_map(conn)?;
            let mut items = Vec::new();
            for platform in all_platforms() {
                let env_key = env_key(platform);
                let env_value = env::var(env_key).ok();
                let db_has = db_map.contains_key(platform.as_str());
                let configured = env_value.is_some() || db_has;
                let source = if env_value.is_some() {
                    "env"
                } else if db_has {
                    "db"
                } else {
                    "none"
                };
                let expires_at = if source == "db" {
                    get_token_expiry(conn, platform)?
                } else {
                    None
                };
                items.push(AuthStatusItem {
                    platform: platform.as_str().to_string(),
                    configured,
                    source: source.to_string(),
                    expires_at,
                });
            }
            print_json_or_text(
                json,
                &ListResponse {
                    ok: true,
                    count: items.len(),
                    items,
                },
                "auth status listed",
            );
            Ok(())
        }
        AuthCommand::Logout(args) => {
            let deleted = delete_token(conn, args.platform)?;
            let message = if deleted {
                format!("token removed for {}", args.platform.as_str())
            } else {
                format!("no stored token for {}", args.platform.as_str())
            };
            print_json_or_text(
                json,
                &ActionResponse {
                    ok: true,
                    message,
                    id: None,
                },
                "token removed",
            );
            Ok(())
        }
        AuthCommand::Login(args) => {
            let url = match args.platform {
                Platform::X => "https://developer.x.com/en/docs/authentication/oauth-2-0",
                Platform::Linkedin => {
                    "https://learn.microsoft.com/en-us/linkedin/shared/authentication/authorization-code-flow"
                }
                Platform::Bluesky => "https://atproto.com/guides/app-passwords",
                Platform::Threads => "https://developers.facebook.com/docs/threads",
                Platform::Reddit => "https://github.com/reddit-archive/reddit/wiki/OAuth2",
            };
            print_json_or_text(
                json,
                &ActionResponse {
                    ok: true,
                    message: format!(
                        "interactive login is not yet embedded. complete oauth externally, then use auth set-token. guide: {url}"
                    ),
                    id: None,
                },
                "use auth set-token after oauth flow",
            );
            Ok(())
        }
    }
}

async fn handle_post(
    conn: &rusqlite::Connection,
    args: PostArgs,
    json: bool,
) -> Result<(), AppError> {
    validate_targets(&args.to)?;
    let client = Client::new();
    let req = PostRequest {
        text: args.text,
        media: args.media,
        title: args.title,
        subreddit: args.subreddit,
    };

    let mut tasks = Vec::new();
    for platform in args.to {
        let token = resolve_token(conn, platform)?;
        let req_cloned = req.clone();
        let client_cloned = client.clone();
        tasks.push(tokio::spawn(async move {
            let name = platform.as_str().to_string();
            match post_to_platform(&client_cloned, platform, &token, &req_cloned).await {
                Ok(ok) => ok,
                Err(err) => PostPlatformResult {
                    platform: name,
                    status: "failed".to_string(),
                    remote_post_id: None,
                    url: None,
                    error: Some(err.to_string()),
                },
            }
        }));
    }

    let mut results = Vec::new();
    for task in tasks {
        let item = task
            .await
            .map_err(|e| AppError::RequestFailed(format!("task join failed: {e}")))?;
        results.push(item);
    }

    let response = PostResponse {
        ok: true,
        count: results.len(),
        results,
    };
    print_json_or_text(json, &response, "post fan-out completed");
    Ok(())
}

fn handle_schedule(
    conn: &rusqlite::Connection,
    args: ScheduleArgs,
    json: bool,
) -> Result<(), AppError> {
    validate_targets(&args.to)?;

    let run_at = DateTime::parse_from_rfc3339(&args.at)
        .map_err(|_| AppError::InvalidArgument("--at must be RFC3339".into()))?
        .with_timezone(&Utc);

    if run_at < Utc::now() {
        return Err(AppError::InvalidArgument(
            "--at must be in the future".into(),
        ));
    }

    let draft = PostDraft {
        text: args.text,
        media_path: args.media,
        title: args.title,
        subreddit: args.subreddit,
    };

    let id = schedule_job(conn, run_at, &draft, &args.to)?;
    print_json_or_text(
        json,
        &ActionResponse {
            ok: true,
            message: "job scheduled".to_string(),
            id: Some(id),
        },
        "job scheduled",
    );
    Ok(())
}

fn handle_queue(conn: &rusqlite::Connection, args: QueueArgs, json: bool) -> Result<(), AppError> {
    match args.command {
        QueueCommand::List(args) => {
            let status = args.status.map(|s| s.as_str().to_string());
            let items = queue_list(conn, status.as_deref())?;
            print_json_or_text(
                json,
                &ListResponse {
                    ok: true,
                    count: items.len(),
                    items,
                },
                "queue listed",
            );
            Ok(())
        }
        QueueCommand::Show(args) => {
            let item = queue_show(conn, &args.id)?.ok_or(AppError::NotFound)?;
            print_json_or_text(json, &ItemResponse { ok: true, item }, "queue item");
            Ok(())
        }
        QueueCommand::Cancel(args) => {
            let found = queue_cancel(conn, &args.id)?;
            if !found {
                return Err(AppError::NotFound);
            }
            print_json_or_text(
                json,
                &ActionResponse {
                    ok: true,
                    message: "job canceled".to_string(),
                    id: Some(args.id),
                },
                "job canceled",
            );
            Ok(())
        }
    }
}

async fn handle_run(
    conn: &rusqlite::Connection,
    args: RunArgs,
    json: bool,
) -> Result<(), AppError> {
    if !args.once && !args.daemon {
        return Err(AppError::InvalidArgument(
            "choose --once or --daemon".into(),
        ));
    }
    if args.once && args.daemon {
        return Err(AppError::InvalidArgument(
            "choose only one of --once or --daemon".into(),
        ));
    }

    if args.once {
        let response = run_once(conn).await?;
        print_json_or_text(json, &response, "run once completed");
        return Ok(());
    }

    loop {
        let response = run_once(conn).await?;
        if !json {
            println!(
                "jobs_processed={} targets_sent={} targets_failed={}",
                response.jobs_processed, response.targets_sent, response.targets_failed
            );
        }

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                print_json_or_text(
                    json,
                    &ActionResponse {
                        ok: true,
                        message: "daemon stopped".to_string(),
                        id: None,
                    },
                    "daemon stopped",
                );
                break;
            }
            _ = tokio::time::sleep(Duration::from_secs(args.interval.max(1))) => {}
        }
    }

    Ok(())
}

async fn run_once(conn: &rusqlite::Connection) -> Result<RunResponse, AppError> {
    let jobs = due_jobs(conn, Utc::now())?;
    let client = Client::new();

    let mut jobs_processed = 0usize;
    let mut targets_sent = 0usize;
    let mut targets_failed = 0usize;

    for job in jobs {
        set_job_running(conn, &job.id)?;
        let req = PostRequest {
            text: job.text.clone(),
            media: job.media_path.clone(),
            title: job.title.clone(),
            subreddit: job.subreddit.clone(),
        };

        for target in &job.targets {
            let platform = parse_platform(target)?;
            let resolved = resolve_token(conn, platform);
            let outcome = match resolved {
                Ok(token) => post_to_platform(&client, platform, &token, &req).await,
                Err(err) => Err(err),
            };

            match outcome {
                Ok(result) => {
                    mark_target_result(
                        conn,
                        &job.post_id,
                        &result.platform,
                        "sent",
                        result.remote_post_id.as_deref(),
                        None,
                    )?;
                    targets_sent += 1;
                }
                Err(err) => {
                    mark_target_result(
                        conn,
                        &job.post_id,
                        platform.as_str(),
                        "failed",
                        None,
                        Some(&err.to_string()),
                    )?;
                    targets_failed += 1;
                }
            }
        }

        finalize_job(conn, &job.id, &job.post_id)?;
        jobs_processed += 1;
    }

    Ok(RunResponse {
        ok: true,
        jobs_processed,
        targets_sent,
        targets_failed,
    })
}

fn resolve_token(conn: &rusqlite::Connection, platform: Platform) -> Result<String, AppError> {
    if let Ok(value) = env::var(env_key(platform)) {
        if !value.trim().is_empty() {
            return Ok(value);
        }
    }

    if let Some(value) = get_token(conn, platform)? {
        if !value.trim().is_empty() {
            return Ok(value);
        }
    }

    Err(AppError::AuthMissing(platform.as_str().to_string()))
}

fn env_key(platform: Platform) -> &'static str {
    match platform {
        Platform::X => "DEE_CROSSPOST_X_TOKEN",
        Platform::Linkedin => "DEE_CROSSPOST_LINKEDIN_TOKEN",
        Platform::Bluesky => "DEE_CROSSPOST_BLUESKY_TOKEN",
        Platform::Threads => "DEE_CROSSPOST_THREADS_TOKEN",
        Platform::Reddit => "DEE_CROSSPOST_REDDIT_TOKEN",
    }
}

fn parse_platform(raw: &str) -> Result<Platform, AppError> {
    match raw {
        "x" => Ok(Platform::X),
        "linkedin" => Ok(Platform::Linkedin),
        "bluesky" => Ok(Platform::Bluesky),
        "threads" => Ok(Platform::Threads),
        "reddit" => Ok(Platform::Reddit),
        _ => Err(AppError::InvalidArgument(format!(
            "unsupported platform: {raw}"
        ))),
    }
}

fn validate_targets(targets: &[Platform]) -> Result<(), AppError> {
    if targets.is_empty() {
        return Err(AppError::InvalidArgument(
            "--to requires at least one platform".into(),
        ));
    }
    Ok(())
}

fn all_platforms() -> [Platform; 5] {
    [
        Platform::X,
        Platform::Linkedin,
        Platform::Bluesky,
        Platform::Threads,
        Platform::Reddit,
    ]
}

fn print_json_or_text<T: serde::Serialize>(json: bool, value: &T, fallback: &str) {
    if json {
        let line = serde_json::to_string(value).unwrap_or_else(|_| {
            r#"{"ok":false,"error":"serialization failed","code":"SERDE_ERROR"}"#.to_string()
        });
        println!("{line}");
    } else {
        println!("{fallback}");
    }
}

fn print_error(err: &AppError) {
    let wants_json = std::env::args().any(|arg| arg == "--json" || arg == "-j");
    if wants_json {
        let line = serde_json::to_string(&ErrorResponse {
            ok: false,
            error: err.to_string(),
            code: err.code().to_string(),
        })
        .unwrap_or_else(|_| {
            r#"{"ok":false,"error":"unexpected error","code":"UNEXPECTED"}"#.to_string()
        });
        println!("{line}");
    } else {
        eprintln!("error: {err}");
    }
}

#[allow(dead_code)]
fn _verify_help_builds() {
    Cli::command();
}
