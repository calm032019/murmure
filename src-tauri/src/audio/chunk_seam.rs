//! Repairs punctuation at silence-cut chunk boundaries.
//!
//! Each chunk is transcribed in isolation, so the model always ends it with a
//! full stop and capitalises the start of the next one, even when the speaker
//! only paused mid-sentence ("...into the revisit. Section on the schedule.").
//! The fix transcribes a short window spanning the cut, where the model can
//! hear both sides, and keeps its verdict on whether the sentence really ended.

use crate::utils::text::normalize_text;

/// Audio kept on each side of a silence cut for the seam re-transcription.
pub(super) const SEAM_CONTEXT_SECS: f32 = 10.0;

fn trailing_punct(word: &str) -> &str {
    let end = word.trim_end_matches(|c: char| !c.is_alphanumeric()).len();
    &word[end..]
}

fn ends_sentence(word: &str) -> bool {
    trailing_punct(word).contains(['.', '?', '!'])
}

fn starts_lowercase(word: &str) -> bool {
    word.chars()
        .find(|c| c.is_alphabetic())
        .is_some_and(char::is_lowercase)
}

fn lowercase_first(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(c) => c.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Given the last word before a cut, the first word after it, and a
/// transcription of the audio spanning the cut, returns the corrected pair when
/// the seam transcription runs the two words together without ending the
/// sentence. Returns None when nothing should change.
pub(super) fn repair_seam(last: &str, first: &str, seam_text: &str) -> Option<(String, String)> {
    if !ends_sentence(last) {
        return None;
    }
    let (last_key, first_key) = (normalize_text(last), normalize_text(first));
    if last_key.is_empty() || first_key.is_empty() {
        return None;
    }
    let seam: Vec<&str> = seam_text.split_whitespace().collect();
    let joined = seam
        .windows(2)
        .find(|pair| normalize_text(pair[0]) == last_key && normalize_text(pair[1]) == first_key)?;
    if ends_sentence(joined[0]) {
        return None; // The model hears a real sentence end here too.
    }

    let stem = last.trim_end_matches(|c: char| !c.is_alphanumeric());
    let fixed_last = format!("{}{}", stem, trailing_punct(joined[0]));
    let fixed_first = if starts_lowercase(joined[1]) && !starts_lowercase(first) {
        lowercase_first(first)
    } else {
        first.to_string()
    };
    Some((fixed_last, fixed_first))
}

/// Replaces the last word of `text`, keeping everything before it.
pub(super) fn replace_last_word(text: &mut String, word: &str) {
    let trimmed = text.trim_end();
    let start = trimmed.rfind(char::is_whitespace).map_or(0, |i| i + 1);
    text.truncate(start);
    text.push_str(word);
}

/// Replaces the first word of `text` when it matches `original`.
pub(super) fn replace_first_word(text: &str, original: &str, word: &str) -> String {
    let trimmed = text.trim_start();
    match trimmed.split_once(char::is_whitespace) {
        Some((head, rest)) if head == original => format!("{} {}", word, rest),
        None if trimmed == original => word.to_string(),
        _ => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_a_pause_mid_sentence() {
        assert_eq!(
            repair_seam(
                "revisit.",
                "Section",
                "pulled through into the non-urgent revisit section on the install"
            ),
            Some(("revisit".to_string(), "section".to_string()))
        );
    }

    #[test]
    fn keeps_a_comma_the_model_hears() {
        assert_eq!(
            repair_seam(
                "Thursday.",
                "But",
                "we meet on Thursday, but not if it rains"
            ),
            Some(("Thursday,".to_string(), "but".to_string()))
        );
    }

    #[test]
    fn keeps_a_real_sentence_end() {
        assert_eq!(
            repair_seam("today.", "Thanks", "I sent it over today. Thanks again"),
            None
        );
    }

    #[test]
    fn keeps_proper_nouns_capitalised() {
        assert_eq!(
            repair_seam("in.", "Leeds", "the job is in Leeds next week"),
            Some(("in".to_string(), "Leeds".to_string()))
        );
    }

    #[test]
    fn leaves_seam_alone_when_words_do_not_line_up() {
        assert_eq!(
            repair_seam("revisit.", "Section", "something else entirely"),
            None
        );
        assert_eq!(repair_seam("revisit", "section", "revisit section"), None);
    }

    #[test]
    fn replaces_words_at_the_edges() {
        let mut acc = String::from("into the revisit.");
        replace_last_word(&mut acc, "revisit");
        assert_eq!(acc, "into the revisit");
        assert_eq!(
            replace_first_word("Section on the schedule.", "Section", "section"),
            "section on the schedule."
        );
        assert_eq!(
            replace_first_word("Section.", "Section.", "section."),
            "section."
        );
        assert_eq!(
            replace_first_word("Other words", "Section", "section"),
            "Other words"
        );
    }
}
