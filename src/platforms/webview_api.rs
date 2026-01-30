use crate::database;
use crate::meta_api::HttpResult;
use crate::meta_api::expose_internal_error;
use crate::plurality;
use crate::updater;
use crate::users;
use anyhow::anyhow;
use rocket::serde::json::Json;
use rocket::{State, response::content::RawHtml};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct GenericFrontingStatus {
    inner: String,
}

#[get("/api/fronting-status")]
pub async fn get_api_fronting_status(
    jwt: users::Jwt,
    db_pool: &State<PgPool>,
    application_user_secrets: &State<database::ApplicationUserSecrets>,
    client: &State<reqwest::Client>,
    shared_updaters: &State<updater::UpdaterManager>,
) -> HttpResult<Json<GenericFrontingStatus>> {
    let user_id = jwt.user_id().map_err(expose_internal_error)?;

    log::debug!("# | GET /api/fronting-status/{user_id}");

    let config =
        database::get_user_config_with_secrets(db_pool, &user_id, client, application_user_secrets)
            .await
            .map_err(expose_internal_error)?;

    log::debug!("# | GET /api/fronting-status/{user_id} | got_config");

    let fronters = shared_updaters
        .fronter_channel_get_most_recent_sent_value(&user_id)
        .map_err(expose_internal_error)?
        .ok_or_else(|| anyhow!("No data from Simply Plural found (2)?"))
        .map_err(expose_internal_error)?;

    log::debug!(
        "# | GET /api/fronting-status/{user_id} | got_config | {} fronts",
        fronters.len()
    );

    let fronting_format = plurality::FrontingFormat {
        cleaning: plurality::CleanForPlatform::NoClean,
        max_length: None,
        prefix: config.status_prefix,
        status_if_no_fronters: config.status_no_fronts,
        truncate_names_to_length_if_status_too_long: config.status_truncate_names_to,
    };

    let as_status = plurality::format_fronting_status(&fronting_format, &fronters);

    let result = GenericFrontingStatus { inner: as_status };

    log::debug!(
        "# | GET /api/fronting-status/{user_id} | got_config | {} fronts | rendered to status string",
        fronters.len()
    );

    Ok(Json(result))
}

#[get("/fronting/<website_url_name>")]
pub async fn get_api_fronting_by_user_id(
    website_url_name: &str,
    db_pool: &State<PgPool>,
    application_user_secrets: &State<database::ApplicationUserSecrets>,
    shared_updaters: &State<updater::UpdaterManager>,
    client: &State<reqwest::Client>,
) -> HttpResult<RawHtml<String>> {
    log::debug!("# | GET /fronting/{website_url_name}");

    let user_info = database::find_user_by_website_url_name(db_pool, website_url_name)
        .await
        .map_err(expose_internal_error)?;
    let user_id = user_info.id;

    log::debug!("# | GET /fronting/{website_url_name} | {user_id}");

    let config =
        database::get_user_config_with_secrets(db_pool, &user_id, client, application_user_secrets)
            .await
            .map_err(expose_internal_error)?;

    log::debug!("# | GET /fronting/{website_url_name} | {user_id} | got_config");

    let fronts = shared_updaters
        .fronter_channel_get_most_recent_sent_value(&user_id)
        .map_err(expose_internal_error)?
        .ok_or_else(|| anyhow!("No data from Simply Plural found?"))
        .map_err(expose_internal_error)?;

    log::debug!(
        "# | GET /fronting/{website_url_name} | {user_id} | got_config | {} fronts",
        fronts.len()
    );

    let html = generate_html(&config.website_system_name, &fronts);

    log::debug!(
        "# | GET /fronting/{website_url_name} | {user_id} | got_config | {} fronts | HTML generated",
        fronts.len()
    );

    Ok(RawHtml(html))
}

fn generate_html(website_system_name: &str, fronts: &[plurality::Fronter]) -> String {
    let fronts_formatted_and_escaped = fronts
        .iter()
        .map(|m| -> String {
            format!(
                "<div><img src=\"{}\" /><p>{}</p></div>",
                html_escape::encode_double_quoted_attribute(&m.avatar_url),
                html_escape::encode_text(&m.name)
            )
        })
        .collect::<Vec<String>>()
        .join("\n");

    format!(
        r"<html>
    <head>
        <title>{} - Fronting Status</title>
        <style>
            /* --- layout container ------------------------------------ */
            body{{
                margin:0;
                padding:1rem;
                font-family:sans-serif;
                display:flex;
                flex-direction: column;
                gap:1rem;
            }}

            /* --- one card -------------------------------------------- */
            body>div {{
                flex:1 1 calc(25% - 1rem);   /* ≤4 cards per row */
                display:flex;
                align-items:center;
                gap:.75rem;
                padding:.75rem;
                background:#fff;
                border-radius:.5rem;
                box-shadow:0 2px 4px rgba(0,0,0,.08);
            }}

            /* --- avatar image ---------------------------------------- */
            body>div img {{
                width:10rem;
                height:10rem;           /* fixed square keeps things tidy */
                object-fit:cover;
                border-radius:50%;
            }}

            /* --- name ------------------------------------------------- */
            body>div p {{
                margin:0;
                font-size: 3rem;
                font-weight:600;
            }}

            /* --- phones & tablets ------------------------------------ */
            @media (max-width:800px) {{
                body>div {{flex:1 1 calc(50% - 1rem);}}   /* 2-across */
            }}
            @media (max-width:420px) {{
                body>div {{flex:1 1 100%;}}               /* stack */
            }}
        </style>
    </head>
    <body>
        {}
    </body>
</html>",
        html_escape::encode_text(website_system_name),
        fronts_formatted_and_escaped
    )
}

#[cfg(test)]
mod tests {
    use super::generate_html;
    use crate::plurality::Fronter;

    #[test]
    fn test_generate_html_escaping() {
        let fronters = vec![Fronter {
            fronter_id: "some-id".to_string(),
            name: "<script>alert('XSS')</script>".to_string(),
            avatar_url: "https://example.com/avatar.png".to_string(),
            vrchat_status_name: None,
            start_time: None,
            privacy_buckets: vec![],
            pluralkit_id: None,
        }];
        let system_name = "My <System>";
        let html = generate_html(system_name, &fronters);

        // Test system name escaping
        assert!(html.contains("<title>My &lt;System&gt; - Fronting Status</title>"));

        // Test fronter name escaping
        assert!(html.contains("<p>&lt;script&gt;alert('XSS')&lt;/script&gt;</p>"));

        // Test avatar url is not escaped (as it should be a URL)
        assert!(html.contains("src=\"https://example.com/avatar.png\""));
    }

    #[test]
    fn test_generate_html_empty_fronters() {
        let fronters = vec![];
        let system_name = "My System";
        let html = generate_html(system_name, &fronters);

        assert!(html.contains("<title>My System - Fronting Status</title>"));
        assert!(!html.contains("<div><img"));
    }

    #[test]
    fn test_generate_html_multiple_fronters() {
        let fronters = vec![
            Fronter {
                fronter_id: "id1".to_string(),
                name: "Fronter 1".to_string(),
                avatar_url: "https://example.com/avatar1.png".to_string(),
                vrchat_status_name: None,
                start_time: None,
                privacy_buckets: vec![],
                pluralkit_id: None,
            },
            Fronter {
                fronter_id: "id2".to_string(),
                name: "Fronter 2".to_string(),
                avatar_url: "https://example.com/avatar2.png".to_string(),
                vrchat_status_name: None,
                start_time: None,
                privacy_buckets: vec![],
                pluralkit_id: None,
            },
        ];
        let system_name = "My System";
        let html = generate_html(system_name, &fronters);

        assert!(html.contains("<p>Fronter 1</p>"));
        assert!(html.contains("src=\"https://example.com/avatar1.png\""));
        assert!(html.contains("<p>Fronter 2</p>"));
        assert!(html.contains("src=\"https://example.com/avatar2.png\""));
    }

    #[test]
    fn test_avatar_url_escaped() {
        let fronters = vec![Fronter {
            fronter_id: "some-id".to_string(),
            name: "Dangerous".to_string(),
            avatar_url: "https://example.com/\" onerror=\"alert('oops')".to_string(),
            vrchat_status_name: None,
            start_time: None,
            privacy_buckets: vec![],
            pluralkit_id: None,
        }];
        let system_name = "My System";
        let html = generate_html(system_name, &fronters);

        assert!(html.contains("src=\"https://example.com/&quot; onerror=&quot;alert('oops')\""));
    }

    #[test]
    fn test_avatar_url_xss_prevented() {
        let fronters = vec![Fronter {
            fronter_id: "some-id".to_string(),
            name: "Hacker".to_string(),
            avatar_url: "\"><script>alert('xss')</script>".to_string(),
            vrchat_status_name: None,
            start_time: None,
            privacy_buckets: vec![],
            pluralkit_id: None,
        }];
        let system_name = "My System";
        let html = generate_html(system_name, &fronters);

        assert!(!html.contains("\"><script>alert('xss')</script>"));
        assert!(html.contains("src=\"&quot;&gt;&lt;script&gt;alert('xss')&lt;/script&gt;\""));
    }
}
