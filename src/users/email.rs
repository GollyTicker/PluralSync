use crate::setup;
use anyhow::Result;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::header::ContentType,
    transport::smtp::authentication::Credentials,
};
use pluralsync_base::users::{Email, PasswordResetToken};

pub async fn send_reset_email(
    smtp_config: &setup::SmtpConfig,
    to: &Email,
    token: &PasswordResetToken,
) -> Result<()> {
    let reset_link = format!(
        "{}/api/auth/reset-password?token={}",
        smtp_config.frontend_base_url, token.inner.inner
    );
    let email_body = format!(
        "Hello,\n\n\
        You have requested to reset your password. Please click the link below to reset it:\n\n\
        {reset_link}\n\n\
        If you did not request this, please ignore this email.\n\n\
        This link will expire in 1 hour."
    );

    let email = Message::builder()
        .from(smtp_config.from_email.parse()?)
        .to(to.inner.parse()?)
        .subject("Password Reset Request")
        .header(ContentType::TEXT_PLAIN)
        .body(email_body)?;

    let creds = Credentials::new(smtp_config.username.clone(), smtp_config.password.clone());

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_config.host)?
        .credentials(creds)
        .port(smtp_config.port)
        .build();

    mailer.send(email).await?;

    Ok(())
}
