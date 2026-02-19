use crate::setup;
use anyhow::Result;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::header::ContentType,
    transport::smtp::authentication::Credentials,
};
use pluralsync_base::users::EmailVerificationToken;
use pluralsync_base::users::{Email, PasswordResetToken};

pub async fn send_reset_email(
    smtp_config: &setup::SmtpConfig,
    to: &Email,
    token: &PasswordResetToken,
) -> Result<()> {
    let reset_link = format!(
        "{}/reset-password?token={}",
        smtp_config.frontend_base_url, token.inner.inner
    );

    send_email(smtp_config, to, "PluralSync Password Reset", format!(
        "Dear PluralSync User,\n\n\
        You have requested to reset your password. Please copy and paste the link below into your browser to reset it:\n\n\
        {reset_link}\n\n\
        If you did not request this, please ignore this email.\n\n\
        This link will expire in 1 hour.\n\n\
        Kinds, PluralSync"
    )).await?;

    Ok(())
}

pub async fn send_verification_email(
    smtp_config: &setup::SmtpConfig,
    to: &Email,
    token: &EmailVerificationToken,
) -> Result<()> {
    let verification_link = format!(
        "{}/verify-email?token={}",
        smtp_config.frontend_base_url, token.inner.inner
    );

    send_email(smtp_config, to, "PluralSync Email Verification", format!(
        "Dear PluralSync User,\n\n\
        Thank you for registering with PluralSync. Please click on the link below to verify your email address:\n\n\
        {verification_link}\n\n\
        This link will expire in 1 hour.\n\n\
        Kinds, PluralSync"
    )).await?;

    Ok(())
}

pub async fn send_email_change_confirmation_link_to_new_email(
    smtp_config: &setup::SmtpConfig,
    to: &Email,
    token: &EmailVerificationToken,
) -> Result<()> {
    let confirmation_link = format!(
        "{}/verify-email?token={}",
        smtp_config.frontend_base_url, token.inner.inner
    );

    send_email(smtp_config, to, "PluralSync Email Change Confirmation", format!(
        "Dear PluralSync User,\n\n\
        You have requested to change your email address to {}. Please click on the link below to confirm this change:\n\n\
        {confirmation_link}\n\n\
        This link will expire in 1 hour.\n\n\
        If you did not request this change, please ignore this email.\n\n\
        Kinds, PluralSync",
        to.inner
    )).await?;

    Ok(())
}

pub async fn send_email_change_notification_to_old_email(
    smtp_config: &setup::SmtpConfig,
    to: &Email,
    new_email: &Email,
) -> Result<()> {
    send_email(smtp_config, to, "PluralSync Email Change Notification", format!(
        "Dear PluralSync User,\n\n\
        This is a notification that your PluralSync account email address has been requested to change from {} to {}.\n\n\
        Kinds, PluralSync",
        to.inner, new_email.inner
    )).await?;

    Ok(())
}

async fn send_email(
    smtp_config: &setup::SmtpConfig,
    to: &Email,
    subject: &str,
    body: String,
) -> Result<()> {
    if smtp_config.dangerous_local_dev_mode_print_tokens_instead_of_send_email {
        log::info!("[DEV MODE - EMAIL NOT SENT] To: {}", to.inner);
        log::info!("[DEV MODE - EMAIL SUBJECT] {subject}");
        log::info!("[DEV MODE - EMAIL BODY]\n{body}");
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

    mailer.send(email).await?;

    Ok(())
}
