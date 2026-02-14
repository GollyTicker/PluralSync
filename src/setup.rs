use crate::database;
use crate::updater;
use crate::users;
use anyhow::Result;

use pluralsync_base::meta;
use pluralsync_base::meta::PLURALSYNC_VERSION;
use rocket::http::Method;
use sqlx::postgres;
use std::env;
use std::time::Duration;

pub const EVERY_MINUTE: &str = "0 * * * * *";
pub const EVERY_5_MINUTES: &str = "*/5 * * * * *";

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

    let client = make_client()?;

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

pub fn make_client() -> Result<reqwest::Client> {
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT))
        .build()?;

    Ok(client)
}

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
}

#[derive(Debug, Clone, Default)]
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
pub async fn start_cron_job<F>(
    db_pool: &sqlx::PgPool,
    shared_updaters: &updater::UpdaterManager,
    application_user_secrets: &database::ApplicationUserSecrets,
    name: &str,
    schedule: &str,
    task: impl (Fn(sqlx::PgPool, updater::UpdaterManager, database::ApplicationUserSecrets) -> F)
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
    let shared_updaters = shared_updaters.clone();
    let application_user_secrets = application_user_secrets.clone();
    let name = name.to_string();
    let job = tokio_cron_scheduler::Job::new(schedule, move |_, _| {
        let db_pool = db_pool.clone();
        let shared_updaters = shared_updaters.clone();
        let application_user_secrets = application_user_secrets.clone();
        let task = task.clone();
        let name = name.clone();
        tokio::spawn(async move {
            if let Err(e) = task(db_pool, shared_updaters, application_user_secrets).await {
                log::error!("Failed to run '{}' job: {e}", &name);
            }
        });
    })?;
    scheduler.add(job).await?;
    scheduler.start().await?;
    Ok(())
}
