//! Starts a new paragraph where the speaker ended a sentence and then paused
//! for a long time: people stop to think before moving to a new subject.
//! Uses the model's token timestamps, so it needs no spoken "new paragraph".

use crate::engine::transcription_engine::TranscriptionSegment;

/// Gap between the last sound of one sentence and the first of the next that
/// starts a paragraph. Measured from token start times, so it includes the
/// last syllable (~0.1-0.3 s) on top of the actual silence.
pub(super) const PARAGRAPH_PAUSE_SECS: f32 = 2.0;

/// Rough length of a word's final token, used to estimate trailing silence.
const LAST_TOKEN_SECS: f32 = 0.2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct WordTiming {
    /// Start of the word's first token.
    pub start: f32,
    /// Start of the word's last spoken (alphanumeric) token.
    pub last: f32,
}

/// Groups token segments into words the same way `word_confidences` does: a
/// token with a leading space starts a word, punctuation-only words are dropped.
pub(super) fn word_timings(segments: &[TranscriptionSegment]) -> Vec<WordTiming> {
    let mut words: Vec<(WordTiming, bool)> = Vec::new();
    for seg in segments {
        let spoken = seg.text.chars().any(char::is_alphanumeric);
        match words.last_mut() {
            Some((timing, has_content)) if !seg.text.starts_with(' ') => {
                if spoken {
                    timing.last = seg.start;
                    *has_content = true;
                }
            }
            _ => words.push((
                WordTiming {
                    start: seg.start,
                    last: seg.start,
                },
                spoken,
            )),
        }
    }
    words
        .into_iter()
        .filter_map(|(timing, has_content)| has_content.then_some(timing))
        .collect()
}

/// Whether this much silence after a finished sentence starts a paragraph.
pub(super) fn is_paragraph_pause(silence_secs: f32) -> bool {
    silence_secs + LAST_TOKEN_SECS >= PARAGRAPH_PAUSE_SECS
}

fn ends_sentence(word: &str) -> bool {
    let end = word.trim_end_matches(|c: char| !c.is_alphanumeric()).len();
    word[end..].contains(['.', '?', '!'])
}

/// Rejoins `text` with a blank line after each sentence followed by a long
/// pause. Leaves the text alone if its words do not line up with the timings.
pub(super) fn insert_pause_breaks(text: &str, timings: &[WordTiming]) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() != timings.len() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + 8);
    for (i, word) in words.iter().enumerate() {
        if i > 0 {
            let gap = timings[i].start - timings[i - 1].last;
            out.push_str(
                if ends_sentence(words[i - 1]) && gap >= PARAGRAPH_PAUSE_SECS {
                    "\n\n"
                } else {
                    " "
                },
            );
        }
        out.push_str(word);
    }
    out
}

/// Silence before the first word and after the last one, given the audio
/// length and the padding added on both sides before inference.
pub(super) fn edge_silence(timings: &[WordTiming], audio_secs: f32, pad_secs: f32) -> (f32, f32) {
    match (timings.first(), timings.last()) {
        (Some(first), Some(last)) => (
            (first.start - pad_secs).max(0.0),
            (audio_secs - pad_secs - last.last - LAST_TOKEN_SECS).max(0.0),
        ),
        _ => (0.0, 0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(text: &str, start: f32) -> TranscriptionSegment {
        TranscriptionSegment {
            start,
            end: start + 0.08,
            text: text.to_string(),
        }
    }

    #[test]
    fn groups_tokens_into_words() {
        let segs = [
            seg(" Hel", 0.0),
            seg("lo", 0.2),
            seg(".", 0.3),
            seg(" ", 0.5),
            seg(" Next", 3.0),
        ];
        assert_eq!(
            word_timings(&segs),
            vec![
                WordTiming {
                    start: 0.0,
                    last: 0.2
                },
                WordTiming {
                    start: 3.0,
                    last: 3.0
                }
            ]
        );
    }

    fn timings(starts: &[f32]) -> Vec<WordTiming> {
        starts
            .iter()
            .map(|&s| WordTiming {
                start: s,
                last: s + 0.1,
            })
            .collect()
    }

    #[test]
    fn long_pause_after_sentence_starts_paragraph() {
        let t = timings(&[0.0, 0.3, 0.6, 3.5, 3.8, 4.1]);
        assert_eq!(
            insert_pause_breaks("I sent it. New topic here.", &t),
            "I sent it.\n\nNew topic here."
        );
    }

    #[test]
    fn short_pause_or_mid_sentence_pause_stays_inline() {
        let t = timings(&[0.0, 0.3, 0.6, 1.4, 1.7]);
        assert_eq!(
            insert_pause_breaks("I sent it. Then more.", &t),
            "I sent it. Then more."
        );
        let t = timings(&[0.0, 0.3, 0.6, 3.5, 3.8]);
        assert_eq!(
            insert_pause_breaks("I sent it to the team.", &t),
            "I sent it to the team."
        );
    }

    #[test]
    fn mismatched_word_count_leaves_text() {
        assert_eq!(
            insert_pause_breaks("One. Two.", &timings(&[0.0])),
            "One. Two."
        );
    }

    #[test]
    fn measures_edge_silence() {
        let t = timings(&[1.25, 2.0]);
        let (lead, trail) = edge_silence(&t, 5.0, 0.25);
        assert!((lead - 1.0).abs() < 1e-4);
        assert!((trail - 2.45).abs() < 1e-4);
    }
}
