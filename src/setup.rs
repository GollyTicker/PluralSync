use crate::database;
use crate::updater;
use crate::users;
use anyhow::Result;

use anyhow::anyhow;
use pluralsync_base::meta;
use pluralsync_base::meta::PLURALSYNC_VERSION;
use rocket::http::Method;
use sqlx::postgres;
use std::env;
use std::sync::OnceLock;
use std::time::Duration;

pub const EVERY_MINUTE: &str = "0 * * * * *";
pub const EVERY_5_MINUTES: &str = "0 */5 * * * *";
pub const DAILY_AT_0400: &str = "0 0 3 * * *";

const REQUEST_TIMEOUT: u64 = 10;

pub fn logging_init() {
    let pluralsync_log_level =
        env::var("PLURALSYNC_LOG_LEVEL").unwrap_or_else(|_| "info".to_owned());
    let log_levels = format!("info,pluralsync={pluralsync_log_level},pluralsync_base=debug");
    println!("Using log levels: {log_levels}");
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_levels))
        .format_timestamp_millis()
        .init();
}

pub async fn application_setup(cli_args: &ApplicationConfig) -> Result<ApplicationSetup> {
    log::info!("# | application_setup");

    let client = global_shared_client()?;

    log::info!("# | application_setup | client_created");

    let jwt_secret = users::ApplicationJwtSecret {
        inner: cli_args.jwt_application_secret.clone(),
    };

    let application_user_secrets = database::ApplicationUserSecrets {
        inner: cli_args.application_user_secrets.clone(),
    };

    let pluralsync_variant_info = meta::PluralSyncVariantInfo {
        version: PLURALSYNC_VERSION.to_owned(),
        variant: cli_args.pluralsync_variant.clone(),
        description: cli_args.pluralsync_variant_description.clone(),
        show_in_ui: !cli_args.pluralsync_variant_hide_in_ui,
    };

    let shared_updaters = updater::UpdaterManager::new(cli_args);

    log::info!("# | application_setup | client_created | basic_info_and_secrets");

    let allowed_origins = rocket_cors::AllowedOrigins::All;
    let allowed_methods = vec![
        Method::Get,
        Method::Post,
        Method::Options,
        Method::Put,
        Method::Head,
    ]
    .into_iter()
    .map(From::from)
    .collect();

    let cors_policy = rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods,
        allowed_headers: rocket_cors::AllowedHeaders::All,
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors()?;

    log::info!("# | application_setup | client_created | basic_info_and_secrets | cors_configured");

    let db_pool = postgres::PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&cli_args.database_url)
        .await?;

    log::info!(
        "# | application_setup | client_created | basic_info_and_secrets | cors_configured | db_connection_created"
    );

    // the macro integrates these files from compile-time!
    let () = sqlx::migrate!("docker/migrations").run(&db_pool).await?;

    log::info!(
        "# | application_setup | client_created | basic_info_and_secrets | cors_configured | db_connection_created | db_migrated"
    );

    let smtp_config = SmtpConfig {
        host: cli_args.smtp_host.clone(),
        port: cli_args.smtp_port,
        username: cli_args.smtp_username.clone(),
        password: cli_args.smtp_password.clone(),
        from_email: cli_args.smtp_from_email.clone(),
        frontend_base_url: cli_args.frontend_base_url.clone(),
        dangerous_local_dev_mode_print_tokens_instead_of_send_email: cli_args
            .dangerous_local_dev_mode_print_tokens_instead_of_send_email,
        email_rate_limit_per_day: cli_args.email_rate_limit_per_day,
    };

    Ok(ApplicationSetup {
        db_pool,
        client,
        pluralsync_variant_info,
        jwt_secret,
        application_user_secrets,
        shared_updaters,
        cors_policy,
        smtp_config,
    })
}

static GLOBAL_SHARED_CLIENT: OnceLock<Result<reqwest::Client, anyhow::Error>> = OnceLock::new();

pub fn global_shared_client() -> Result<reqwest::Client> {
    let result = GLOBAL_SHARED_CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .cookie_store(true)
                .timeout(Duration::from_secs(REQUEST_TIMEOUT))
                .build()
                .map_err(|e| anyhow!("Failed to build global shared client: {e}"))
        });
    match result {
        Ok(c) => Ok(c.clone()),
        Err(e) => Err(anyhow!("{}", e)),
    }
}

#[derive(Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub frontend_base_url: String,
    pub dangerous_local_dev_mode_print_tokens_instead_of_send_email: bool,
    pub email_rate_limit_per_day: u32,
}

#[derive(Clone, Default)]
pub struct ApplicationConfig {
    pub database_url: String,
    pub request_timeout: u64,
    pub pluralsync_variant: String,
    pub pluralsync_variant_description: Option<String>,
    pub pluralsync_variant_hide_in_ui: bool,
    pub jwt_application_secret: String,
    pub application_user_secrets: String,
    pub discord_status_message_updater_available: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_from_email: String,
    pub frontend_base_url: String,
    pub dangerous_local_dev_mode_print_tokens_instead_of_send_email: bool,
    pub email_rate_limit_per_day: u32,
}

impl ApplicationConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: env::var("DATABASE_URL")?,
            request_timeout: env::var("REQUEST_TIMEOUT")
                .unwrap_or_else(|_| "5".to_string())
                .parse()?,
            pluralsync_variant: env::var("PLURALSYNC_VARIANT")?,
            pluralsync_variant_description: env::var("PLURALSYNC_VARIANT_DESCRIPTION").ok(),
            pluralsync_variant_hide_in_ui: env::var("PLURALSYNC_VARIANT_HIDE_IN_UI")
                .unwrap_or_else(|_| "false".to_string())
                .parse()?,
            jwt_application_secret: env::var("JWT_APPLICATION_SECRET")?,
            application_user_secrets: env::var("APPLICATION_USER_SECRETS")?,
            discord_status_message_updater_available: env::var(
                "DISCORD_STATUS_MESSAGE_UPDATER_AVAILABLE",
            )
            .unwrap_or_else(|_| "false".to_string())
            .parse()?,
            smtp_host: env::var("SMTP_HOST")?,
            smtp_port: env::var("SMTP_PORT")?.parse()?,
            smtp_username: env::var("SMTP_USERNAME")?,
            smtp_password: env::var("SMTP_PASSWORD")?,
            smtp_from_email: env::var("SMTP_FROM_EMAIL")?,
            frontend_base_url: env::var("FRONTEND_BASE_URL")?,
            dangerous_local_dev_mode_print_tokens_instead_of_send_email: env::var(
                "DANGEROUS_LOCAL_DEV_MODE_PRINT_TOKENS_INSTEAD_OF_SEND_EMAIL",
            )
            .unwrap_or_else(|_| "false".to_string())
            .parse()?,
            email_rate_limit_per_day: env::var("SMTP_EMAIL_RATE_LIMIT_PER_DAY")
                .unwrap_or_else(|_| "300".to_string())
                .parse()?,
        })
    }
}

#[derive(Clone)]
pub struct ApplicationSetup {
    pub db_pool: sqlx::PgPool,
    pub client: reqwest::Client,
    pub pluralsync_variant_info: meta::PluralSyncVariantInfo,
    pub jwt_secret: users::ApplicationJwtSecret,
    pub application_user_secrets: database::ApplicationUserSecrets,
    pub shared_updaters: updater::UpdaterManager,
    pub cors_policy: rocket_cors::Cors,
    pub smtp_config: SmtpConfig,
}

/* Yes, this signature is daunting, but essentially it's just taking a task: Fn(PgPool) -> Future<Result<()>>.
The many extra traits are simply what rustc recommended to make this work, and it works!
*/
#[allow(clippy::too_many_arguments)]
pub async fn start_cron_job<F>(
    db_pool: &sqlx::PgPool,
    client: &reqwest::Client,
    shared_updaters: &updater::UpdaterManager,
    application_user_secrets: &database::ApplicationUserSecrets,
    smtp_config: &SmtpConfig,
    name: &str,
    schedule: &str,
    task: impl (Fn(
        sqlx::PgPool,
        reqwest::Client,
        updater::UpdaterManager,
        database::ApplicationUserSecrets,
        SmtpConfig,
    ) -> F)
    + Send
    + Sync
    + 'static
    + Clone,
) -> Result<()>
where
    F: Future<Output = Result<()>> + Send,
{
    let scheduler = tokio_cron_scheduler::JobScheduler::new().await?;
    let db_pool = db_pool.clone();
    let client = client.clone();
    let shared_updaters = shared_updaters.clone();
    let application_user_secrets = application_user_secrets.clone();
    let smtp_config = smtp_config.clone();
    let name = name.to_string();
    let job = tokio_cron_scheduler::Job::new(schedule, move |_, _| {
        let db_pool = db_pool.clone();
        let client = client.clone();
        let shared_updaters = shared_updaters.clone();
        let application_user_secrets = application_user_secrets.clone();
        let smtp_config = smtp_config.clone();
        let task = task.clone();
        let name = name.clone();
        tokio::spawn(async move {
            if let Err(e) = task(
                db_pool,
                client,
                shared_updaters,
                application_user_secrets,
                smtp_config,
            )
            .await
            {
                log::error!("Failed to run '{}' job: {e}", &name);
            }
        });
    })?;
    scheduler.add(job).await?;
    scheduler.start().await?;
    Ok(())
}
