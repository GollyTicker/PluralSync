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
        "{}/reset-password?token={}",
        smtp_config.frontend_base_url, token.inner.inner
    );
    let email_body = format!(
        "Dear PluralSync User,\n\n\
        You have requested to reset your password. Please copy and paste the link below into your browser to reset it:\n\n\
        {reset_link}\n\n\
        If you did not request this, please ignore this email.\n\n\
        This link will expire in 1 hour.\n\n\
        Kinds, PluralSync</p>"
    );

    let email = Message::builder()
        .from(smtp_config.from_email.parse()?)
        .to(to.inner.parse()?)
        .subject("PluralSync Password Reset")
        .header(ContentType::TEXT_PLAIN)
        .body(email_body)?;

    let creds = Credentials::new(smtp_config.username.clone(), smtp_config.password.clone());

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp_config.host)?
        .credentials(creds)
        .port(smtp_config.port)
        .build();

    mailer.send(email).await?;

    Ok(())
}
