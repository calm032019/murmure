use unicode_normalization::UnicodeNormalization;

/// Lowercase, strip accents and punctuation, collapse whitespace.
pub(crate) fn normalize_text(text: &str) -> String {
    text.to_lowercase()
        .nfd()
        // NFD decomposes é into e + \u{0301}; filter out the combining marks
        .filter(|c| !('\u{0300}'..='\u{036F}').contains(c))
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .map(|w| match w {
            "okay" => "ok",
            "alice" => "alix",
            _ => w,
        })
        .collect::<Vec<&str>>()
        .join(" ")
}
