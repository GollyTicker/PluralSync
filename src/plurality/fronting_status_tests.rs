use crate::plurality::{
    CleanForPlatform, Fronter, FrontingFormat, VRCHAT_MAX_ALLOWED_STATUS_LENGTH,
    format_fronting_status, fronting_status::string_unicode_codepoints_length,
};

fn mock_formatter_for_tests(
    prefix: &str,
    no_fronts: &str,
    name_truncate_to: usize,
    max_length: usize,
) -> FrontingFormat {
    FrontingFormat {
        prefix: prefix.to_owned(),
        status_if_no_fronters: no_fronts.to_owned(),
        truncate_names_to_length_if_status_too_long: name_truncate_to,
        cleaning: CleanForPlatform::VRChat,
        max_length: Some(max_length),
    }
}

// Helper function to create mock MemberContent
fn mock_member_content(name: &str, _unused: &str) -> Fronter {
    Fronter {
        fronter_id: String::new(),
        name: name.to_string(),
        pronouns: None,
        avatar_url: String::new(),
        start_time: None,
        privacy_buckets: vec![],
        pluralkit_id: None,
    }
}

#[test]
fn test_format_vrchat_status_empty_fronts() {
    let config = mock_formatter_for_tests("F:", "nobody?", 3, VRCHAT_MAX_ALLOWED_STATUS_LENGTH);
    let fronts = vec![];
    assert_eq!(format_fronting_status(&config, &fronts), "F: nobody?");
}

#[test]
fn test_format_vrchat_status_single_member_fits_long_string() {
    let config = mock_formatter_for_tests("F:", "N/A", 3, VRCHAT_MAX_ALLOWED_STATUS_LENGTH);
    let fronts = vec![mock_member_content("Alice", "")]; // "P: Alice" (8 chars)
    assert_eq!(format_fronting_status(&config, &fronts), "F: Alice");
}

#[test]
fn test_format_vrchat_status_multiple_members_fit_long_string() {
    let config = mock_formatter_for_tests("F:", "N/A", 3, VRCHAT_MAX_ALLOWED_STATUS_LENGTH);
    let fronts = vec![
        mock_member_content("Alice", ""),
        mock_member_content("Bob", ""),
    ]; // "P: Alice, Bob" (13 chars)
    assert_eq!(format_fronting_status(&config, &fronts), "F: Alice, Bob");
}

#[test]
fn test_format_vrchat_status_fits_short_string_not_long() {
    // VRCHAT_MAX_ALLOWED_STATUS_LENGTH is 23
    let config = mock_formatter_for_tests("Status:", "N/A", 3, VRCHAT_MAX_ALLOWED_STATUS_LENGTH);
    let fronts = vec![
        mock_member_content("UserOne", ""),
        mock_member_content("UserTwo", ""),
    ];
    // Long: "Status: UserOne, UserTwo" (24 chars) > 23
    // Short: "Status:UserOne,UserTwo" (23 chars) <= 23
    assert_eq!(
        format_fronting_status(&config, &fronts),
        "Status:UserOne,UserTwo"
    );
}

#[test]
fn test_format_vrchat_status_fits_truncated_string_not_short() {
    let config = mock_formatter_for_tests("F:", "N/A", 3, VRCHAT_MAX_ALLOWED_STATUS_LENGTH);
    let fronts = vec![
        mock_member_content("Alexander", ""),
        mock_member_content("Benjamin", ""),
        mock_member_content("Charlotte", ""),
    ];
    // Long: "P: Alexander, Benjamin, Charlotte" 33 > 23
    // Short: "P:Alexander,Benjamin,Charlotte" 31 > 23
    // Truncated: "P:Ale,Ben,Cha" 14 <= 23
    assert_eq!(format_fronting_status(&config, &fronts), "F:Ale,Ben,Cha");
}

#[test]
fn test_format_vrchat_status_uses_name() {
    let config = mock_formatter_for_tests("F:", "N/A", 3, VRCHAT_MAX_ALLOWED_STATUS_LENGTH);
    let fronts = vec![mock_member_content("OriginalName", "")];
    assert_eq!(format_fronting_status(&config, &fronts), "F: OriginalName");
}

#[test]
fn test_format_vrchat_status_cleans_names() {
    let config = mock_formatter_for_tests("F:", "N/A", 3, VRCHAT_MAX_ALLOWED_STATUS_LENGTH);
    let fronts = vec![mock_member_content("User😊Name", "")];
    assert_eq!(format_fronting_status(&config, &fronts), "F: UserName");
}

#[test]
fn test_format_vrchat_status_doesnt_clean_names_when_not_needed() {
    let config = mock_formatter_for_tests("F:", "N/A", 3, VRCHAT_MAX_ALLOWED_STATUS_LENGTH);
    let fronts = vec![mock_member_content("UserName", "")];
    assert_eq!(format_fronting_status(&config, &fronts), "F: UserName");
}

#[test]
fn test_format_vrchat_status_complex_truncation() {
    let config = mock_formatter_for_tests("F:", "N/A", 4, VRCHAT_MAX_ALLOWED_STATUS_LENGTH);
    let fronts = vec![
        mock_member_content("LongNameOne😊", ""),
        mock_member_content("Shorty", ""),
        mock_member_content("AnotherVeryLong", ""),
    ];
    // Cleaned names for status: LongNameOne, Shorty, AnotherVeryLong
    // Long: "F: LongNameOne, Shorty, AnotherVeryLong" 40 > 23
    // Short: "F:LongNameOne,Shorty,AnotherVeryLong" 38 > 23
    // Truncated names: Long, Shor, Anot
    // Truncated string: "F:Long,Shor,Anot" 17 <= 23
    assert_eq!(format_fronting_status(&config, &fronts), "F:Long,Shor,Anot");
}

#[test]
fn test_format_status_truncation() {
    let config = mock_formatter_for_tests("F:", "N/A", 4, 10);
    let fronts = vec![
        mock_member_content("LongNameOne😊", ""),
        mock_member_content("Shorty", ""),
        mock_member_content("AnotherVeryLong", ""),
    ];
    // Cleaned names for status: LongNameOne, Shorty, AnotherVeryLong
    // Truncated names: Long, Shor, Anot
    // Truncated string: "F:Long,Shor,Anot" 17 > 10
    // Count: "F: 3#" 5 <= 10
    assert_eq!(format_fronting_status(&config, &fronts), "F: 3#");
}

#[test]
fn length_counts_codepoints_and_not_bytes() {
    assert_eq!(string_unicode_codepoints_length("123"), 3);
    assert_eq!(string_unicode_codepoints_length("é"), 1);
    assert_eq!(string_unicode_codepoints_length("你好"), 2);
}
