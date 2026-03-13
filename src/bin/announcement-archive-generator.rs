use anyhow::Result;
use pluralsync::database::UserInfo;
use pluralsync::users::{
    SecretHashString, UserId,
    announcement_email::{AnnouncementEmail, get_all_announcement_emails},
};
use pluralsync_base::users::Email;
use serde::Serialize;
use std::fs;

const DESTINATION: &str = "./frontend/public/announcements.json";

/// Represents a rendered announcement email for static display
#[derive(Serialize)]
struct RenderedAnnouncement {
    email_id: String,
    date: String,
    subject: String,
    body: String,
}

/// Test user data for rendering announcements
struct TestUser {
    email: &'static str,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl Default for TestUser {
    fn default() -> Self {
        Self {
            email: "test-user@pluralsync.example.com",
            created_at: chrono::DateTime::from_timestamp_nanos(0),
        }
    }
}

/// Render an announcement email with test user data
fn render_announcement(email: &AnnouncementEmail, test_user: &TestUser) -> RenderedAnnouncement {
    // Create a minimal UserInfo-like struct for rendering
    // We use a simple approach: just pass test data through the subject_fn and body_fn
    let user_info = UserInfo {
        id: UserId::from(uuid::Uuid::nil()),
        email: Email::from(test_user.email.to_string()),
        password_hash: SecretHashString {
            inner: "$dummy$hash$for$rendering$only".to_string(),
        },
        created_at: test_user.created_at,
        new_email: None,
        email_verification_token_hash: None,
        email_verification_token_expires_at: None,
    };

    RenderedAnnouncement {
        email_id: email.email_id.to_string(),
        date: email.date.to_string(),
        subject: (email.subject_fn)(&user_info),
        body: (email.body_fn)(&user_info),
    }
}

fn main() -> Result<()> {
    println!("Generating announcement archive to {DESTINATION}...");

    let emails = get_all_announcement_emails();
    let test_user = TestUser::default();

    let mut rendered: Vec<RenderedAnnouncement> = emails
        .iter()
        .map(|email| render_announcement(email, &test_user))
        .collect();

    // Sort by date (newest first)
    rendered.sort_by(|a, b| b.date.cmp(&a.date));

    let json = serde_json::to_string_pretty(&rendered)?;
    fs::write(DESTINATION, json)?;

    println!("Generated {} announcements.", rendered.len());
    println!("Done.");
    Ok(())
}
