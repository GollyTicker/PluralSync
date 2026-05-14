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
    /// Date of the announcement in ISO format (YYYY-MM-DD), set by the coder
    pub date: &'static str,
    pub subject_fn: fn(&database::UserInfo) -> String,
    pub body_fn: fn(&database::UserInfo) -> String,
}

#[must_use]
pub fn email_announcements_activated() -> AnnouncementEmail {
    AnnouncementEmail {
        email_id: "2026-03-email-announcements-activated",
        date: "2026-03-11",
        subject_fn: |_user| "PluralSync 🔄 Email Announcements to All Users Activated".to_string(),
        body_fn: |_user| {
            "Dear PluralSync Users,\n\n\
            As of today (2026-03-11), PluralSync will start sending essential announcement emails concerning the service and the users usage.\n\n\
            These emails cannot be deactivated because they're essential.\n\n\
            Any additional information or discussion can be had in the corresponding community spaces (see website footer for discord link).\n\n\
            Thank you for using PluralSync. We kindly appreciate and wish you a pleasent and useful time with it.\n\n\
            Kinds, PluralSync".to_owned()
        },
    }
}

#[must_use]
pub fn smiply_plural_discontinuation_1() -> AnnouncementEmail {
    AnnouncementEmail {
        email_id: "2026-03-simply_plural_discontinuation",
        date: "2026-03-12",
        subject_fn: |_user| {
            "PluralSync 🔄 - Regarding the Discontinuation of SimplyPlural".to_string()
        },
        body_fn: |_user| {
            "Dear PluralSync Users,\n\
            \n\
            Unfortunately, recently it was announced that SimplyPlural will be discontinued ( https://apparyllis.com/simply-plural-will-be-discontinued/ ).\n\
            We're very sad to hear that and we have deep empathy with the developer(s) - as we're aware of the complexities of maintaining an open source project used by many many people.\n\
            We're deeply thankful for the existence of SimplyPlural since that's a core reason PluralSync was created in the first place.\n\
            \n\
            At the same time, we're aware, that many PluralSync users deeply depend on SimplyPlural. In that regards, we want to clarify,\n\
            how PluralSync will move in future given the recent announcement.\n\
            \n\
            As the developers of PluralSync, we're watching how the situation will develop.\n\
            Perhaps someone will take over and continue the server maintenance for SimplyPlural.\n\
            Perhaps the community will move to one or two main alternatives. Or maybe SimplyPlural will become self-hosted by many.\n\
            \n\
            Either way PluralSync will keep on functioning and add integrations so that syncing will keep on working.\n\
            Only SimplyPlural-related functionality in PluralSync will stop working once the SP servers are shutdown. \n\
            Since the situation is new and fresh, things might change so this is not a 100% statement on what will happen.\n\
            But we can tell you that we're *planning* to continue PluralSync in this way. The new integrations will be whatever the community at large decided to use.\n\
            The software behind PluralSync is currently focused on SimplyPlural - but we are planning to separate that. When that will happen depends on our time and energy and we don't make any promises currently.\n\
            \n\
            As developers of PluralSync, we've also taken note of how the community has reacted towards the sudden announcement.\n\
            While it's a stressful situation for many, we'd like to remind everybody, that the SimplyPlural developers have poured their heart and soul and money\n\
            and stress over a long time for this - and this is also true for the other developers (PluralKit, Octocon, etc.).\n\
            Please be kind to such developers. They do these things, because of love - and earn nothing to little from these projects\n\
            while taking responsibilities and risks. Please be kind to them and absolutely avoid demanding things - because\n\
            at the end of the day, there is no reason for others to expect anything, when developers offer the software and service for FREE (for most users).\n\
            A kinder atmosphere will make it easier for software to be further developed - but a harsh atmosphere will move developers away.\n\
            Finally, please keep rumors and gossip away from such discussions. Online these days, it's easy to take quick conclusions based on limited data.\n\
            Please don't jump to conclusions. It's better to simply note things and be less confident of other peoples malice. Ignorance or genuine misunderstandings are far more commonly the actual cause.\n\
            \n\
            \n\
            Thanks you and deep respect and gratitude to the developer(s) of SimplyPlural for everything they've done for this community! ❤️
            \n\
            \n\
            Kinds, PluralSync".to_owned()
        },
    }
}

#[must_use]
pub fn pluralkit_as_source() -> AnnouncementEmail {
    AnnouncementEmail {
        email_id: "2026-03-29-pluralkit-added",
        date: "2026-03-29",
        subject_fn: |_| "PluralSync 🔄 - Added PluralKit as Syncing Source 💖".to_string(),
        body_fn: |_| {
            "Dear PluralSync Users,\n\
            \n\
            Being able to synchronize from PluralKit to other platforms and system managers was a frequently requested feature from the beginning of PluralSync.\n
            We're happy to announce, that from now on, you can sync Pluralkit -> * ! ❤️\n\
            \n\
            Check the settings to configure the synchronisation. The synchronisation respects the privacy levels of the fields and the member visibility - and it can be configured as well.\n\
            This integration is still fresh - so issues might arise. If you have issues or have feedback, let us know on the community server!\n\
            Due to technical details, the requests against PluralKit are slowed down a bit - but this should be resolved in a few weeks and not be noticeable for you practically.\n\
            \n\
            Given the near-future discontinuation of SimplyPlural ( https://apparyllis.com/simply-plural-will-be-discontinued/ ), having more options on PluralSync is important.\n\
            \n\
            We're very happy for the PluralKit community and especially the devs for their interactions with us. We enjoy the open-source availability of PluralKit and\n\
            that also enabled us to contribute new code to PluralKit which is needed for PluralSync to better support this synchronisation sustainabily.\n\
            \n\
            Thank you for your attenion.\n\
            \n\
            Kinds, PluralSync
            "
            .to_owned()
        },
    }
}

#[must_use]
pub fn developer_absence_in_june() -> AnnouncementEmail {
    AnnouncementEmail {
        email_id: "2026-05-14-dev-aabsence",
        date: "2026-03-29",
        subject_fn: |_| "Developer Hiatus in June and Possibly Longer".to_string(),
        body_fn: |_| {
            "Dear PluralSync Users,\n\
            \n\
            The developer(s) are taking a break from almost all activities due to personal reasons starting June 2026. The break was planned a long time ago already and is not
            related to the current situation regarding plural system apps. The devs will return from anywhere between 2 weeks to 1.5 months.\n\
            \n\
            The Pluralsync servers will be continuously running during this duration. There won't be any new features during this break. In the community spaces, non-developers are still
            there and they can help with certain issues if you have any. The servers run stable by themselves without the developers' active hand.
            \n\
            Thank you for your attention.\n\
            \n\
            Kinds, PluralSync
            "
            .to_owned()
        },
    }
}

/// Registry of all announcement emails
/// Add new emails here when deploying
#[must_use]
pub fn get_all_announcement_emails() -> Vec<AnnouncementEmail> {
    vec![
        email_announcements_activated(),
        smiply_plural_discontinuation_1(),
        pluralkit_as_source(),
        developer_absence_in_june()
        // todo. add announcement about asking for donations
        // todo. add announcement about SP shutdown
    ]
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
    let user_info = database::get_user_info(db_pool, &user_id).await?;
    let subject = (email_def.subject_fn)(&user_info);
    let body = (email_def.body_fn)(&user_info);
    let to = user_info.email;
    match users::email::send_email_with_threshold(
        db_pool,
        smtp_config,
        &to,
        &subject,
        body,
        rate_limit_threshold,
        users::email::EmailType::Announcement,
    )
    .await
    {
        Ok(()) => {
            database::record_announcement_email_success(db_pool, &user_id, &email_id).await?;
            log::info!("Sent announcement email '{email_id}' to user {user_id}");
        }
        Err(e) => {
            // Check if this is a rate limit threshold error - if so, stop processing
            if e.to_string()
                .contains("EMAIL_RATE_LIMIT_THRESHOLD_EXCEEDED")
            {
                log::warn!("Skipping announcement email (rate limit threshold reached): {e}");
                return Ok(LoopStreamControl::Break);
            }
            // Other errors: record failure and continue
            database::record_announcement_email_failure(db_pool, &user_id, &email_id).await?;
            log::warn!("Failed to send announcement email '{email_id}' to user {user_id}: {e}");
        }
    }
    Ok(LoopStreamControl::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Test Constants ===
    const TEST_EMAIL_ID: &str = "2026-03-email-announcements-activated";
    const DEFAULT_RETRY_DELAY_HOURS: i64 = 4;
    const DEFAULT_RATE_THRESHOLD: f64 = 0.8;
    const FULL_RATE_THRESHOLD: f64 = 1.0;
    const TEST_PASSWORD_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$test$test";
    const EMAIL_DEFINITION_OFFSET_HOURS: i64 = 1;

    // === Helper Functions ===

    /// Create a base test SMTP config (dev mode, no actual sending)
    fn test_smtp_config() -> setup::SmtpConfig {
        setup::SmtpConfig {
            email_rate_limit_per_day: 100,
            dangerous_local_dev_mode_print_tokens_instead_of_send_email: true,
            host: "smtp.test.com".to_string(),
            port: 587,
            username: "test".to_string(),
            password: "test".to_string(),
            from_email: "test@test.com".to_string(),
            frontend_base_url: "http://test.local".to_string(),
        }
    }

    /// Create a rate-limited SMTP config for testing rate limit behavior
    fn rate_limited_smtp_config(limit: u32) -> setup::SmtpConfig {
        setup::SmtpConfig {
            email_rate_limit_per_day: limit,
            ..test_smtp_config()
        }
    }

    /// Create a test user in the database
    async fn create_test_user(pool: &PgPool, email: &str) -> Result<UserId, anyhow::Error> {
        let user_id = sqlx::query_scalar::<_, uuid::Uuid>(
            "INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING id",
        )
        .bind(email)
        .bind(TEST_PASSWORD_HASH)
        .fetch_one(pool)
        .await?;

        Ok(UserId { inner: user_id })
    }

    /// Get user's created_at timestamp
    async fn get_user_created_at(
        pool: &PgPool,
        user_id: &UserId,
    ) -> Result<chrono::DateTime<chrono::Utc>, sqlx::Error> {
        sqlx::query_scalar("SELECT created_at FROM users WHERE id = $1")
            .bind(user_id.inner)
            .fetch_one(pool)
            .await
    }

    /// Set up an announcement email definition that makes users eligible to receive it
    async fn setup_announcement_email_for_users(
        pool: &PgPool,
        user_created_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), anyhow::Error> {
        // Create email definition with created_at AFTER the users registered
        // This makes the users eligible to receive the email
        // (users registered BEFORE the email was created should receive it)
        sqlx::query(
            "INSERT INTO announcement_email_definitions (email_id, created_at) VALUES ($1, $2)",
        )
        .bind(TEST_EMAIL_ID)
        .bind(user_created_at + chrono::Duration::hours(EMAIL_DEFINITION_OFFSET_HOURS))
        .execute(pool)
        .await?;

        // Delete the definition so ensure_announcement_email_definitions will create it fresh
        // This is necessary because ensure only adds pending emails for newly created definitions
        sqlx::query!(
            "DELETE FROM announcement_email_definitions WHERE email_id = $1",
            TEST_EMAIL_ID
        )
        .execute(pool)
        .await?;

        // Create pending entries for eligible users
        let emails = get_all_announcement_emails();
        database::ensure_announcement_email_definitions(pool, emails.as_slice()).await?;
        Ok(())
    }

    /// Assert that a pending email entry exists
    async fn assert_pending_email_exists(
        pool: &PgPool,
        user_id: &UserId,
        email_id: &str,
        message: &str,
    ) {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pending_emails WHERE user_id = $1 AND email_id = $2)",
        )
        .bind(user_id.inner)
        .bind(email_id)
        .fetch_one(pool)
        .await
        .expect("Failed to query pending email existence");
        assert!(exists, "{message}");
    }

    /// Assert that a pending email entry does not exist
    async fn assert_pending_email_not_exists(
        pool: &PgPool,
        user_id: &UserId,
        email_id: &str,
        message: &str,
    ) {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pending_emails WHERE user_id = $1 AND email_id = $2)",
        )
        .bind(user_id.inner)
        .bind(email_id)
        .fetch_one(pool)
        .await
        .expect("Failed to query pending email existence");
        assert!(!exists, "{message}");
    }

    // === Tests ===

    #[test]
    fn test_get_all_announcement_emails_returns_non_empty_vec() {
        // === Arrange ===
        // No setup needed

        // === Act ===
        let emails = get_all_announcement_emails();

        // === Assert ===
        assert!(
            !emails.is_empty(),
            "Should return at least one announcement email"
        );
        assert_eq!(
            emails[0].email_id, TEST_EMAIL_ID,
            "First email should be welcome announcement"
        );
    }

    #[sqlx::test(migrations = "docker/migrations")]
    async fn test_send_pending_announcement_emails_empty_queue(
        pool: PgPool,
    ) -> Result<(), anyhow::Error> {
        // === Arrange ===
        let smtp_config = test_smtp_config();

        // === Act ===
        let result = send_pending_announcement_emails(
            &pool,
            &smtp_config,
            DEFAULT_RATE_THRESHOLD,
            DEFAULT_RETRY_DELAY_HOURS,
        )
        .await;

        // === Assert ===
        assert!(result.is_ok(), "Should succeed with empty queue");

        Ok(())
    }

    #[sqlx::test(migrations = "docker/migrations")]
    async fn test_send_pending_announcement_emails_rate_limit_stops_sending(
        pool: PgPool,
    ) -> Result<(), anyhow::Error> {
        // === Arrange ===
        // Reset the rate limit table for this test
        sqlx::query!("DELETE FROM email_rate_limit WHERE id = 1")
            .execute(&pool)
            .await?;

        // Create test users
        let user1_id = create_test_user(&pool, "user1@test.com").await?;
        let _user2_id = create_test_user(&pool, "user2@test.com").await?;
        let _user3_id = create_test_user(&pool, "user3@test.com").await?;

        // Get user created_at and set up announcement email
        let user_created_at = get_user_created_at(&pool, &user1_id).await?;
        setup_announcement_email_for_users(&pool, user_created_at).await?;

        // Get all announcement email IDs to calculate total pending emails
        let all_emails = get_all_announcement_emails();
        let num_email_types = all_emails.len();
        let total_users = 3;
        let total_pending_emails = total_users * num_email_types;

        // Set rate limit to allow sending exactly (total_pending_emails - 1) emails
        // This ensures exactly 1 email remains pending after rate limiting stops sending
        // Note: We need to account for all email types, not just TEST_EMAIL_ID
        let rate_limit = (total_pending_emails - 1) as u32;
        let smtp_config = rate_limited_smtp_config(rate_limit);

        // === Act ===
        send_pending_announcement_emails(
            &pool,
            &smtp_config,
            FULL_RATE_THRESHOLD,
            DEFAULT_RETRY_DELAY_HOURS,
        )
        .await?;

        // === Assert ===
        // Should have sent (total_pending_emails - 1) emails and stopped
        // So exactly 1 email should remain pending across all types
        // Since emails are processed in random order, we can't guarantee which type remains
        // But we know exactly 1 email total remains pending
        let total_remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pending_emails")
            .fetch_one(&pool)
            .await?;

        assert_eq!(
            total_remaining, 1,
            "Should stop sending after rate limit is reached, leaving exactly 1 email pending"
        );

        Ok(())
    }

    #[sqlx::test(migrations = "docker/migrations")]
    async fn test_send_pending_announcement_emails_success_removes_from_pending(
        pool: PgPool,
    ) -> Result<(), anyhow::Error> {
        // === Arrange ===
        // Reset the rate limit table for this test
        sqlx::query!("DELETE FROM email_rate_limit WHERE id = 1")
            .execute(&pool)
            .await?;

        // Create test user
        let user_id = create_test_user(&pool, "success-test@test.com").await?;

        // Get user created_at and set up announcement email
        let user_created_at = get_user_created_at(&pool, &user_id).await?;
        setup_announcement_email_for_users(&pool, user_created_at).await?;

        // Verify it exists before sending
        assert_pending_email_exists(
            &pool,
            &user_id,
            TEST_EMAIL_ID,
            "Pending email should exist before sending",
        )
        .await;

        let smtp_config = test_smtp_config();

        // === Act ===
        send_pending_announcement_emails(
            &pool,
            &smtp_config,
            FULL_RATE_THRESHOLD,
            DEFAULT_RETRY_DELAY_HOURS,
        )
        .await?;

        // === Assert ===
        assert_pending_email_not_exists(
            &pool,
            &user_id,
            TEST_EMAIL_ID,
            "Pending email should be removed after successful send",
        )
        .await;

        Ok(())
    }
}
