use crate::metrics::{
    EMAIL_RATE_LIMIT_CURRENT_COUNT, EMAIL_RATE_LIMIT_EXCEEDED_TOTAL, EMAIL_SEND_FAILURE_TOTAL,
    EMAIL_SEND_SUCCESS_TOTAL, EMAILS_SENT_TOTAL,
};
use crate::{database, setup};
use anyhow::Result;
use chrono::Utc;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::header::ContentType,
    transport::smtp::authentication::Credentials,
};
use pluralsync_base::users::{Email, EmailVerificationToken, PasswordResetToken};
use sqlx::PgPool;
use strum_macros::Display;

#[derive(Display)]
pub enum EmailType {
    Verification,
    PasswordReset,
    EmailChangeConfirmation,
    EmailChangeNotification,
    AccountDeletion,
    Announcement,
}

pub async fn send_reset_email(
    db_pool: &PgPool,
    smtp_config: &setup::SmtpConfig,
    to: &Email,
    token: &PasswordResetToken,
) -> Result<()> {
    let reset_link = format!(
        "{}/reset-password?token={}",
        smtp_config.frontend_base_url, token.inner.inner
    );

    send_email(
        db_pool,
        smtp_config,
        to,
        "PluralSync 🔄 Password Reset",
        format!(
            "Dear PluralSync Users,\n\n\
        You have requested to reset your password. Please copy and paste the link below into your browser to reset it:\n\n\
        {reset_link}\n\n\
        If you did not request this, please ignore this email.\n\n\
        This link will expire in 1 hour.\n\n\
        Kinds, PluralSync"
        ),
        EmailType::PasswordReset,
    )
    .await?;

    Ok(())
}

pub async fn send_verification_email(
    db_pool: &PgPool,
    smtp_config: &setup::SmtpConfig,
    to: &Email,
    token: &EmailVerificationToken,
) -> Result<()> {
    let verification_link = format!(
        "{}/verify-email?token={}",
        smtp_config.frontend_base_url, token.inner.inner
    );

    send_email(
        db_pool,
        smtp_config,
        to,
        "Welcome to PluralSync 🔄 ❤️ - Verify Your Email",
        format!(
            "Dear PluralSync Users,\n\n\
        Thank you for registering with PluralSync. Please click on the link below to verify your email address:\n\n\
        {verification_link}\n\n\
        This link will expire in 1 hour.\n\n\
        Kinds, PluralSync"
        ),
        EmailType::Verification,
    )
    .await?;

    Ok(())
}

pub async fn send_email_change_confirmation_link_to_new_email(
    db_pool: &PgPool,
    smtp_config: &setup::SmtpConfig,
    to: &Email,
    token: &EmailVerificationToken,
) -> Result<()> {
    let confirmation_link = format!(
        "{}/verify-email?token={}",
        smtp_config.frontend_base_url, token.inner.inner
    );

    send_email(
        db_pool,
        smtp_config,
        to,
        "Confirm Your New PluralSync 🔄 Email",
        format!(
            "Dear PluralSync Users,\n\n\
        You have requested to change your email address to {}. Please click on the link below to confirm this change:\n\n\
        {confirmation_link}\n\n\
        This link will expire in 1 hour.\n\n\
        If you did not request this change, please ignore this email.\n\n\
        Kinds, PluralSync",
            to.inner
        ),
        EmailType::EmailChangeConfirmation,
    )
    .await?;

    Ok(())
}

pub async fn send_email_change_notification_to_old_email(
    db_pool: &PgPool,
    smtp_config: &setup::SmtpConfig,
    to: &Email,
    new_email: &Email,
) -> Result<()> {
    send_email(
        db_pool,
        smtp_config,
        to,
        "Your PluralSync 🔄 Email Was Requested To Be Changed",
        format!(
            "Dear PluralSync Users,\n\n\
        This is a notification that your PluralSync account email address has been requested to change from {} to {}.\n\n\
        Kinds, PluralSync",
            to.inner, new_email.inner
        ),
        EmailType::EmailChangeNotification,
    )
    .await?;

    Ok(())
}

pub async fn send_account_deletion_notification(
    db_pool: &PgPool,
    smtp_config: &setup::SmtpConfig,
    to: &Email,
) -> Result<()> {
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    send_email(
        db_pool,
        smtp_config,
        to,
        "Thanks For Having Used PluralSync 🔄 ❤️ - Your Account Is Deleted",
        format!(
            "Dear PluralSync Users,\n\n\
            This email confirms that your PluralSync account has been permanently deleted.\n\n\
            Deletion timestamp: {timestamp}\n\n\
            All your data, including authentication tokens, platform credentials and updaters \
            have been removed from our servers. Your account cannot be recovered.\n\n\
            Kinds, PluralSync"
        ),
        EmailType::AccountDeletion,
    )
    .await?;

    Ok(())
}

pub async fn send_email(
    db_pool: &PgPool,
    smtp_config: &setup::SmtpConfig,
    to: &Email,
    subject: &str,
    body: String,
    email_type: EmailType,
) -> Result<()> {
    send_email_with_threshold(db_pool, smtp_config, to, subject, body, 1.0, email_type).await
}

pub async fn send_email_with_threshold(
    db_pool: &PgPool,
    smtp_config: &setup::SmtpConfig,
    to: &Email,
    subject: &str,
    body: String,
    rate_limit_threshold: f64,
    email_type: EmailType,
) -> Result<()> {
    let email_type_str = email_type.to_string();

    // Increment emails sent counter
    EMAILS_SENT_TOTAL
        .with_label_values(&[&email_type_str])
        .inc();

    // Check rate limit with threshold before sending
    match database::try_acquire_email_slot(
        db_pool,
        smtp_config.email_rate_limit_per_day,
        Some(rate_limit_threshold),
    )
    .await
    {
        Ok(()) => {}
        Err(e) => {
            log::warn!("Email rate limit exceeded for {}: {}", to.inner, e);
            EMAIL_RATE_LIMIT_EXCEEDED_TOTAL.with_label_values(&[]).inc();
            EMAIL_SEND_FAILURE_TOTAL
                .with_label_values(&[&email_type_str, "rate_limit"])
                .inc();
            return Err(e);
        }
    }

    // Update rate limit current count gauge
    let current_count: i64 = sqlx::query_scalar("SELECT count FROM email_rate_limit WHERE id = 1")
        .fetch_optional(db_pool)
        .await
        .unwrap_or(Some(0))
        .unwrap_or(0);
    EMAIL_RATE_LIMIT_CURRENT_COUNT
        .with_label_values(&[])
        .set(current_count);

    if smtp_config.dangerous_local_dev_mode_print_tokens_instead_of_send_email {
        log::info!("[DEV MODE - EMAIL NOT SENT] To: {}", to.inner);
        log::info!("[DEV MODE - EMAIL SUBJECT] {subject}");
        log::info!("[DEV MODE - EMAIL BODY]\n{body}");
        EMAIL_SEND_SUCCESS_TOTAL
            .with_label_values(&[&email_type_str])
            .inc();
        return Ok(());
    }

    let email = Message::builder()
        .from(smtp_config.from_email.parse()?)
        .to(to.inner.parse()?)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body)?;

    let creds = Credentials::new(smtp_config.username.clone(), smtp_config.password.clone());

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp_config.host)?
        .credentials(creds)
        .port(smtp_config.port)
        .build();

    match mailer.send(email).await {
        Ok(_) => {
            EMAIL_SEND_SUCCESS_TOTAL
                .with_label_values(&[&email_type_str])
                .inc();
        }
        Err(e) => {
            let error_reason = e.to_string();
            EMAIL_SEND_FAILURE_TOTAL
                .with_label_values(&[&email_type_str, &error_reason])
                .inc();
            return Err(e.into());
        }
    }

    Ok(())
}
