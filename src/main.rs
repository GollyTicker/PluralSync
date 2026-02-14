use anyhow::Result;
use anyhow::anyhow;

use pluralsync::meta_api;
use pluralsync::metrics;
use pluralsync::platforms;
use pluralsync::setup;
use pluralsync::updater;
use pluralsync::users;
use pluralsync_base::license;
use rocket::routes;

#[tokio::main]
async fn main() -> Result<()> {
    setup::logging_init();

    println!("{}", license::info_text());

    log::debug!("# | app_setup");

    let application_config = setup::ApplicationConfig::from_env()?;

    let app_setup = setup::application_setup(&application_config).await?;

    log::debug!("# | app_setup | configured");

    let () = updater::api::restart_all_user_updaters_for_app_startups(app_setup.clone()).await?;

    log::debug!("# | app_setup | configured | updaters_restarted");

    let () = setup::start_cron_job(
        &app_setup.db_pool,
        &app_setup.shared_updaters,
        &app_setup.application_user_secrets,
        "user-metrics",
        setup::EVERY_5_MINUTES,
        metrics::collect_user_metrics,
    )
    .await?;

    let () = setup::start_cron_job(
        &app_setup.db_pool,
        &app_setup.shared_updaters,
        &app_setup.application_user_secrets,
        "restart-long-living-updaters",
        setup::EVERY_MINUTE,
        updater::restart_first_long_living_updater,
    )
    .await?;

    log::debug!(
        "# | app_setup | configured | updaters_restarted | cron_jobs_started | webserver_starting"
    );

    log::debug!(
        "# | app_setup | configured | updaters_restarted | metrics_ccron_jobs_startedron_started | webserver_starting"
    );

    let () = run_webserver(app_setup).await?;

    log::debug!(
        "# | app_setup | configured | updaters_restarted | cron_jobs_started | webserver_starting | webserver_ended"
    );

    Ok(())
}

async fn run_webserver(setup: setup::ApplicationSetup) -> Result<()> {
    let _ = rocket::build()
        .manage(setup.db_pool)
        .manage(setup.jwt_secret)
        .manage(setup.application_user_secrets)
        .manage(setup.client)
        .manage(setup.shared_updaters)
        .manage(setup.pluralsync_variant_info)
        .manage(setup.smtp_config)
        .attach(metrics::PROM_METRICS.clone())
        .attach(setup.cors_policy)
        .mount(
            "/",
            routes![
                users::user_api::post_api_user_register,
                users::user_api::post_api_user_login,
                users::user_api::get_api_user_info,
                users::config_api::get_api_user_config,
                users::config_api::post_api_user_config,
                users::config_api::get_api_config_defaults,
                updater::api::get_api_updaters_status,
                platforms::webview_api::get_api_fronting_status,
                platforms::webview_api::get_api_fronting_by_user_id,
                platforms::vrchat_api::post_api_user_platform_vrchat_auth_2fa_request,
                platforms::vrchat_api::post_api_user_platform_vrchat_auth_2fa_resolve,
                platforms::discord_api::get_api_user_platform_discord_bridge_events,
                meta_api::get_api_meta_pluralsync_variant,
            ],
        )
        .mount("/metrics", metrics::PROM_METRICS.clone())
        .launch()
        .await
        .map_err(|e| anyhow!(e))?;

    Ok(())
}
