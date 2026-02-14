use crate::setup;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::header::ContentType,
    transport::smtp::authentication::Credentials,
};

pub async fn send_reset_email(
    smtp_config: &setup::SmtpConfig,
    frontend_url: &str,
    to: &str,
    token: &str,
) -> Result<(), anyhow::Error> {
    let reset_link = format!("{frontend_url}/reset-password?token={token}");
    let email_body = format!(
        "Hello,\n\n\
        You have requested to reset your password. Please click the link below to reset it:\n\n\
        {reset_link}\n\n\
        If you did not request this, please ignore this email.\n\n\
        This link will expire in 1 hour."
    );

    let email = Message::builder()
        .from(smtp_config.from_email.parse()?)
        .to(to.parse()?)
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
