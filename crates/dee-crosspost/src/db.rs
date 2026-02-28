use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use dirs::data_local_dir;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::cli::Platform;
use crate::error::AppError;
use crate::models::{PostPlatformResult, QueueDetail, QueueItem};

#[derive(Debug, Clone)]
pub struct PostDraft {
    pub text: String,
    pub media_path: Option<String>,
    pub title: Option<String>,
    pub subreddit: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScheduledJob {
    pub id: String,
    pub post_id: String,
    pub text: String,
    pub media_path: Option<String>,
    pub title: Option<String>,
    pub subreddit: Option<String>,
    pub targets: Vec<String>,
}

pub fn db_path() -> Result<PathBuf, AppError> {
    let base = data_local_dir().ok_or(AppError::DataDirMissing)?;
    Ok(base.join("dee-crosspost").join("crosspost.db"))
}

pub fn connect(path: &Path) -> Result<Connection, AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| AppError::DataDirMissing)?;
    }
    let conn = Connection::open(path).map_err(|_| AppError::Database)?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS posts (
            id TEXT PRIMARY KEY,
            text TEXT NOT NULL,
            media_path TEXT,
            title TEXT,
            subreddit TEXT,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS post_targets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            post_id TEXT NOT NULL,
            platform TEXT NOT NULL,
            status TEXT NOT NULL,
            error TEXT,
            remote_post_id TEXT,
            attempts INTEGER NOT NULL DEFAULT 0,
            last_attempt_at TEXT,
            UNIQUE(post_id, platform),
            FOREIGN KEY(post_id) REFERENCES posts(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS scheduled_jobs (
            id TEXT PRIMARY KEY,
            post_id TEXT NOT NULL,
            run_at TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY(post_id) REFERENCES posts(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS auth_accounts (
            platform TEXT PRIMARY KEY,
            auth_type TEXT NOT NULL,
            access_token TEXT NOT NULL,
            refresh_token TEXT,
            expires_at TEXT,
            updated_at TEXT NOT NULL
        );
        ",
    )
    .map_err(|_| AppError::Database)?;
    Ok(())
}

pub fn upsert_token(conn: &Connection, platform: Platform, token: &str) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO auth_accounts(platform, auth_type, access_token, updated_at)
         VALUES(?1, 'token', ?2, ?3)
         ON CONFLICT(platform) DO UPDATE SET
             auth_type='token',
             access_token=excluded.access_token,
             updated_at=excluded.updated_at",
        params![platform.as_str(), token, now],
    )
    .map_err(|_| AppError::Database)?;
    Ok(())
}

pub fn delete_token(conn: &Connection, platform: Platform) -> Result<bool, AppError> {
    let changed = conn
        .execute(
            "DELETE FROM auth_accounts WHERE platform=?1",
            params![platform.as_str()],
        )
        .map_err(|_| AppError::Database)?;
    Ok(changed > 0)
}

pub fn get_token(conn: &Connection, platform: Platform) -> Result<Option<String>, AppError> {
    conn.query_row(
        "SELECT access_token FROM auth_accounts WHERE platform=?1",
        params![platform.as_str()],
        |row| row.get(0),
    )
    .optional()
    .map_err(|_| AppError::Database)
}

pub fn get_token_expiry(conn: &Connection, platform: Platform) -> Result<Option<String>, AppError> {
    let value = conn
        .query_row(
            "SELECT expires_at FROM auth_accounts WHERE platform=?1",
            params![platform.as_str()],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|_| AppError::Database)?;
    Ok(value.flatten())
}

pub fn schedule_job(
    conn: &Connection,
    run_at: DateTime<Utc>,
    draft: &PostDraft,
    targets: &[Platform],
) -> Result<String, AppError> {
    let post_id = Uuid::new_v4().to_string();
    let job_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO posts(id, text, media_path, title, subreddit, created_at) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
        params![post_id, draft.text, draft.media_path, draft.title, draft.subreddit, now],
    )
    .map_err(|_| AppError::Database)?;

    for platform in targets {
        conn.execute(
            "INSERT INTO post_targets(post_id, platform, status) VALUES(?1, ?2, 'pending')",
            params![post_id, platform.as_str()],
        )
        .map_err(|_| AppError::Database)?;
    }

    conn.execute(
        "INSERT INTO scheduled_jobs(id, post_id, run_at, status, created_at, updated_at) VALUES(?1, ?2, ?3, 'pending', ?4, ?4)",
        params![job_id, post_id, run_at.to_rfc3339(), now],
    )
    .map_err(|_| AppError::Database)?;

    Ok(job_id)
}

pub fn queue_list(conn: &Connection, status: Option<&str>) -> Result<Vec<QueueItem>, AppError> {
    let mut items = Vec::new();
    let mut stmt = if status.is_some() {
        conn.prepare(
            "SELECT j.id, j.run_at, j.status, p.text, p.id
             FROM scheduled_jobs j
             JOIN posts p ON p.id = j.post_id
             WHERE j.status = ?1
             ORDER BY j.run_at ASC",
        )
    } else {
        conn.prepare(
            "SELECT j.id, j.run_at, j.status, p.text, p.id
             FROM scheduled_jobs j
             JOIN posts p ON p.id = j.post_id
             ORDER BY j.run_at ASC",
        )
    }
    .map_err(|_| AppError::Database)?;

    let mut rows = if let Some(s) = status {
        stmt.query(params![s]).map_err(|_| AppError::Database)?
    } else {
        stmt.query([]).map_err(|_| AppError::Database)?
    };

    while let Some(row) = rows.next().map_err(|_| AppError::Database)? {
        let post_id: String = row.get(4).map_err(|_| AppError::Database)?;
        let targets = targets_for_post(conn, &post_id)?;
        items.push(QueueItem {
            id: row.get(0).map_err(|_| AppError::Database)?,
            run_at: row.get(1).map_err(|_| AppError::Database)?,
            status: row.get(2).map_err(|_| AppError::Database)?,
            text: row.get(3).map_err(|_| AppError::Database)?,
            targets,
        });
    }

    Ok(items)
}

pub fn queue_show(conn: &Connection, id: &str) -> Result<Option<QueueDetail>, AppError> {
    let row = conn
        .query_row(
            "SELECT j.id, j.run_at, j.status, p.id, p.text, p.media_path, p.title, p.subreddit
             FROM scheduled_jobs j
             JOIN posts p ON p.id = j.post_id
             WHERE j.id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            },
        )
        .optional()
        .map_err(|_| AppError::Database)?;

    if let Some((job_id, run_at, status, post_id, text, media_path, title, subreddit)) = row {
        let targets = target_states_for_post(conn, &post_id)?;
        return Ok(Some(QueueDetail {
            id: job_id,
            run_at,
            status,
            text,
            media_path,
            title,
            subreddit,
            targets,
        }));
    }

    Ok(None)
}

pub fn queue_cancel(conn: &Connection, id: &str) -> Result<bool, AppError> {
    let post_id = conn
        .query_row(
            "SELECT post_id FROM scheduled_jobs WHERE id=?1",
            params![id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|_| AppError::Database)?;

    let Some(post_id) = post_id else {
        return Ok(false);
    };

    conn.execute(
        "UPDATE scheduled_jobs SET status='canceled', updated_at=?2 WHERE id=?1 AND status IN ('pending','running')",
        params![id, Utc::now().to_rfc3339()],
    )
    .map_err(|_| AppError::Database)?;

    conn.execute(
        "UPDATE post_targets SET status='canceled' WHERE post_id=?1 AND status='pending'",
        params![post_id],
    )
    .map_err(|_| AppError::Database)?;

    Ok(true)
}

pub fn due_jobs(conn: &Connection, now: DateTime<Utc>) -> Result<Vec<ScheduledJob>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT j.id, j.post_id, j.run_at, p.text, p.media_path, p.title, p.subreddit
             FROM scheduled_jobs j
             JOIN posts p ON p.id = j.post_id
             WHERE j.status = 'pending' AND j.run_at <= ?1
             ORDER BY j.run_at ASC",
        )
        .map_err(|_| AppError::Database)?;
    let mut rows = stmt
        .query(params![now.to_rfc3339()])
        .map_err(|_| AppError::Database)?;
    let mut jobs = Vec::new();

    while let Some(row) = rows.next().map_err(|_| AppError::Database)? {
        let post_id: String = row.get(1).map_err(|_| AppError::Database)?;
        jobs.push(ScheduledJob {
            id: row.get(0).map_err(|_| AppError::Database)?,
            post_id: post_id.clone(),
            text: row.get(3).map_err(|_| AppError::Database)?,
            media_path: row.get(4).map_err(|_| AppError::Database)?,
            title: row.get(5).map_err(|_| AppError::Database)?,
            subreddit: row.get(6).map_err(|_| AppError::Database)?,
            targets: targets_for_post(conn, &post_id)?,
        });
    }

    Ok(jobs)
}

pub fn set_job_running(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE scheduled_jobs SET status='running', updated_at=?2 WHERE id=?1",
        params![id, Utc::now().to_rfc3339()],
    )
    .map_err(|_| AppError::Database)?;
    Ok(())
}

pub fn mark_target_result(
    conn: &Connection,
    post_id: &str,
    platform: &str,
    status: &str,
    remote_post_id: Option<&str>,
    error: Option<&str>,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE post_targets
         SET status=?3, remote_post_id=?4, error=?5, attempts=attempts+1, last_attempt_at=?6
         WHERE post_id=?1 AND platform=?2",
        params![
            post_id,
            platform,
            status,
            remote_post_id,
            error,
            Utc::now().to_rfc3339()
        ],
    )
    .map_err(|_| AppError::Database)?;
    Ok(())
}

pub fn finalize_job(conn: &Connection, id: &str, post_id: &str) -> Result<(), AppError> {
    let (pending, failed): (i64, i64) = conn
        .query_row(
            "SELECT
                SUM(CASE WHEN status='pending' THEN 1 ELSE 0 END),
                SUM(CASE WHEN status='failed' THEN 1 ELSE 0 END)
             FROM post_targets WHERE post_id=?1",
            params![post_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| AppError::Database)?;

    let final_status = if pending > 0 {
        "pending"
    } else if failed > 0 {
        "failed"
    } else {
        "done"
    };

    conn.execute(
        "UPDATE scheduled_jobs SET status=?2, updated_at=?3 WHERE id=?1",
        params![id, final_status, Utc::now().to_rfc3339()],
    )
    .map_err(|_| AppError::Database)?;
    Ok(())
}

fn targets_for_post(conn: &Connection, post_id: &str) -> Result<Vec<String>, AppError> {
    let mut stmt = conn
        .prepare("SELECT platform FROM post_targets WHERE post_id=?1 ORDER BY id ASC")
        .map_err(|_| AppError::Database)?;
    let rows = stmt
        .query_map(params![post_id], |row| row.get::<_, String>(0))
        .map_err(|_| AppError::Database)?;

    let mut targets = Vec::new();
    for row in rows {
        targets.push(row.map_err(|_| AppError::Database)?);
    }
    Ok(targets)
}

fn target_states_for_post(
    conn: &Connection,
    post_id: &str,
) -> Result<Vec<PostPlatformResult>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT platform, status, remote_post_id, error
             FROM post_targets
             WHERE post_id=?1
             ORDER BY id ASC",
        )
        .map_err(|_| AppError::Database)?;
    let rows = stmt
        .query_map(params![post_id], |row| {
            Ok(PostPlatformResult {
                platform: row.get(0)?,
                status: row.get(1)?,
                remote_post_id: row.get(2)?,
                url: None,
                error: row.get(3)?,
            })
        })
        .map_err(|_| AppError::Database)?;

    let mut targets = Vec::new();
    for row in rows {
        targets.push(row.map_err(|_| AppError::Database)?);
    }
    Ok(targets)
}

pub fn auth_status_db_map(
    conn: &Connection,
) -> Result<std::collections::HashMap<String, String>, AppError> {
    let mut stmt = conn
        .prepare("SELECT platform, expires_at FROM auth_accounts")
        .map_err(|_| AppError::Database)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            ))
        })
        .map_err(|_| AppError::Database)?;

    let mut out = std::collections::HashMap::new();
    for row in rows {
        let (platform, expires) = row.map_err(|_| AppError::Database)?;
        out.insert(platform, expires);
    }
    Ok(out)
}
