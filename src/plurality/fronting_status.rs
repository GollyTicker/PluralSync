use crate::{metric, metrics::SHOULDNT_HAPPEN_BUT_IT_DID, plurality::Fronter};

use encoding_rs::ISO_8859_15;

pub const VRCHAT_MAX_ALLOWED_STATUS_LENGTH: usize = 23;
pub const DISCORD_STATUS_MAX_LENGTH: usize = 128;

metric!(
    rocket_prometheus::prometheus::IntCounterVec,
    FRONTING_STATUS_STRING,
    "fronting_status_string",
    &["length"]
);

pub struct FrontingFormat {
    pub max_length: Option<usize>,
    pub cleaning: CleanForPlatform,
    pub prefix: String,
    pub status_if_no_fronters: String,
    pub truncate_names_to_length_if_status_too_long: usize,
}

pub enum CleanForPlatform {
    NoClean,
    VRChat,
}

#[must_use]
pub fn format_fronting_status(fronting_format: &FrontingFormat, fronts: &[Fronter]) -> String {
    let cleaned_fronter_names = collect_clean_fronter_names(fronting_format, fronts);
    log::debug!("# | format_fronting_status | cleaned to '{cleaned_fronter_names:?}'");

    let status_strings =
        compute_status_strings_of_decreasing_lengths_for_aesthetics_and_information_tradeoff(
            fronting_format,
            &cleaned_fronter_names,
        );

    let status =
        pick_longest_string_within_vrchat_status_length_limit(fronting_format, &status_strings);

    FRONTING_STATUS_STRING
        .with_label_values(&[&status.len().to_string()])
        .inc();

    status
}

fn collect_clean_fronter_names(
    fronting_format: &FrontingFormat,
    fronts: &[Fronter],
) -> Vec<String> {
    if fronts.is_empty() {
        vec![fronting_format.status_if_no_fronters.clone()] // Use configured string if no fronters
    } else {
        fronts
            .iter()
            .map(|f| match fronting_format.cleaning {
                CleanForPlatform::NoClean => f.name.clone(),
                CleanForPlatform::VRChat => clean_name_for_vrchat_status(&f.name),
            })
            .collect()
    }
}

fn compute_status_strings_of_decreasing_lengths_for_aesthetics_and_information_tradeoff(
    fronting_format: &FrontingFormat,
    fronter_names: &[String],
) -> Vec<String> {
    // Convert Vec<String> to Vec<&str> for convenient joining and slicing.
    let fronter_names_as_str: Vec<&str> = fronter_names.iter().map(String::as_str).collect();

    let long_string = format!(
        "{} {}",
        fronting_format.prefix,
        fronter_names_as_str.join(", ")
    );
    let short_string = format!(
        "{}{}",
        fronting_format.prefix,
        fronter_names_as_str.join(",")
    );
    let truncated_string = {
        let truncated_names: Vec<String> = fronter_names_as_str
            .iter()
            .map(|&name| {
                let mut truncated_name = String::new();

                let () = &name
                    .chars()
                    .take(fronting_format.truncate_names_to_length_if_status_too_long)
                    .for_each(|c| truncated_name.push(c));

                truncated_name
            })
            .collect();
        format!(
            "{}{}",
            fronting_format.prefix,
            truncated_names.join(",").as_str()
        )
    };
    let count_string = format!("{} {}#", fronting_format.prefix, fronter_names.len());

    log::debug!(
        "# | fronting_status_lengths | long:  '{}' ({})",
        long_string,
        string_unicode_codepoints_length(&long_string)
    );
    log::debug!(
        "# | fronting_status_lengths | short: '{}' ({})",
        short_string,
        string_unicode_codepoints_length(&short_string)
    );
    log::debug!(
        "# | fronting_status_lengths | trunc: '{}' ({})",
        truncated_string,
        string_unicode_codepoints_length(&truncated_string)
    );
    log::debug!(
        "# | fronting_status_lengths | count: '{}' ({})",
        count_string,
        string_unicode_codepoints_length(&count_string)
    );

    vec![long_string, short_string, truncated_string, count_string]
}

fn pick_longest_string_within_vrchat_status_length_limit(
    fronting_format: &FrontingFormat,
    status_strings: &[String],
) -> String {
    let empty_string = String::new();
    status_strings
        .iter()
        .filter(|s| {
            fronting_format
                .max_length
                .is_none_or(|l| string_unicode_codepoints_length(*s) <= l)
        })
        .max_by_key(|s| string_unicode_codepoints_length(*s))
        .unwrap_or_else(|| {
            SHOULDNT_HAPPEN_BUT_IT_DID
                .with_label_values(&["pick_longest_string_within_vrchat_status_length_limit"])
                .inc();
            &empty_string
        })
        .clone()
}

pub fn string_unicode_codepoints_length<S: Into<String>>(s: S) -> usize {
    s.into().chars().count() // doesn'T count graphemes, but I think that should still work well.
}

// VRChat status messages does not display all UTF-8 characters.
// This function removes all characters which are not of a specific encoding from the string.
// We also trim the name, in case the cleanup made new spaces appear.
#[must_use]
pub fn clean_name_for_vrchat_status(dirty_name: &str) -> String {
    dirty_name
        .chars()
        .filter(|&ch| {
            let mut buf = [0u8; 4];
            let s = ch.encode_utf8(&mut buf);
            let mut out = [0u8; 20];
            let (_, _, _, unsupported) = ISO_8859_15
                .new_encoder()
                .encode_from_utf8(s, &mut out, true);

            !unsupported
        })
        .collect::<String>()
        .split_whitespace() // remove consecutive whitespace resulting from cleanup. also trims string.
        .collect::<Vec<&str>>()
        .join(" ")
}
