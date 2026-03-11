use crate::database;
use crate::setup;
use crate::users;
use crate::users::UserId;
use anyhow::Result;
use futures::never::Never;
use pluralsync_base::controlflow::LoopStreamControl;
use sqlx::PgPool;

/// Represents an announcement email definition
pub struct AnnouncementEmail {
    /// Stable, globally unique identifier (e.g., "welcome-announcement-march-2026")
    pub email_id: &'static str,
    pub subject_fn: fn(&database::UserInfo) -> String,
    pub body_fn: fn(&database::UserInfo) -> String,
}

/// Example: Welcome announcement email
#[must_use]
pub fn welcome_announcement_march_2026() -> AnnouncementEmail {
    AnnouncementEmail {
        email_id: "welcome-announcement-march-2026",
        subject_fn: |_user| "Welcome to PluralSync! 🎉".to_string(),
        body_fn: |user| {
            format!(
                "Hi there,\n\n\
                Welcome to PluralSync! You registered on {}.\n\n\
                Kinds, PluralSync",
                user.created_at.format("%B %Y")
            )
        },
    }
}

/// Registry of all announcement emails
/// Add new emails here when deploying
#[must_use]
pub fn get_all_announcement_emails() -> Vec<AnnouncementEmail> {
    vec![welcome_announcement_march_2026()]
}

/// Main entry point: send pending announcement emails
///
/// # Arguments
/// * `db_pool` - Database connection pool
/// * `smtp_config` - SMTP configuration for sending emails
/// * `rate_limit_threshold` - Fraction of quota reserved for priority emails (e.g., 0.8 = 80%)
/// * `retry_delay_hours` - Hours to wait before retrying failed sends (e.g., 4)
#[allow(clippy::needless_continue)]
pub async fn send_pending_announcement_emails(
    db_pool: &PgPool,
    smtp_config: &setup::SmtpConfig,
    rate_limit_threshold: f64,
    retry_delay_hours: i64,
) -> Result<()> {
    log::debug!("# | send_pending_announcement_emails | Starting announcement email sender");

    let emails = get_all_announcement_emails();
    database::ensure_announcement_email_definitions(db_pool, &emails).await?;

    let pending = database::get_pending_announcement_emails(db_pool, retry_delay_hours).await?;

    log::debug!(
        "# | send_pending_announcement_emails | Found {} pending emails",
        pending.len()
    );

    for (user_id, email_id) in pending {
        match send_pending_email(
            db_pool,
            smtp_config,
            rate_limit_threshold,
            &emails,
            user_id.clone(),
            email_id.clone(),
        )
        .await
        {
            Ok(LoopStreamControl::Break) => break,
            Ok(LoopStreamControl::Continue) => continue,
            Err(e) => {
                log::warn!("Failed to send pending email: {user_id} {email_id}. {e}");
                continue;
            }
        }
    }

    Ok(())
}

async fn send_pending_email(
    db_pool: &sqlx::Pool<sqlx::Postgres>,
    smtp_config: &setup::SmtpConfig,
    rate_limit_threshold: f64,
    emails: &[AnnouncementEmail],
    user_id: UserId,
    email_id: String,
) -> Result<LoopStreamControl<Never>, anyhow::Error> {
    let email_def = emails
        .iter()
        .find(|e| e.email_id == email_id)
        .ok_or_else(|| anyhow::anyhow!("Unknown email_id: {email_id}"))?;
    let user_info = database::get_user_info(db_pool, user_id.clone()).await?;
    if let Err(e) = database::try_acquire_email_slot(
        db_pool,
        smtp_config.email_rate_limit_per_day,
        Some(rate_limit_threshold),
    )
    .await
    {
        log::warn!("Skipping announcement email (rate limit threshold reached): {e}");
        return Ok(LoopStreamControl::Break);
    }
    let subject = (email_def.subject_fn)(&user_info);
    let body = (email_def.body_fn)(&user_info);
    let to = user_info.email;
    match users::email::send_email(db_pool, smtp_config, &to, &subject, body).await {
        Ok(()) => {
            database::record_announcement_email_success(db_pool, &user_id, &email_id).await?;
            log::info!("Sent announcement email '{email_id}' to user {user_id}");
        }
        Err(e) => {
            database::record_announcement_email_failure(db_pool, &user_id, &email_id).await?;
            log::warn!("Failed to send announcement email '{email_id}' to user {user_id}: {e}");
        }
    }
    Ok(LoopStreamControl::Continue)
}
