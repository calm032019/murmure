//! Deterministic "intelligence" applied to raw dictation, before the LLM and
//! formatting rules. Three passes, each independently switchable:
//!
//! 1. Filler removal: "Um, so I think, uh, yes." -> "So I think, yes."
//!    Unlike the bare word filter in `clean_transcription`, punctuation and
//!    capitalisation around the removed word are repaired.
//! 2. Voice commands: "new line" / "new paragraph" become line breaks.
//! 3. Self-corrections: "I want an apple, actually I want a pear" ->
//!    "I want a pear"; "Book it for Tuesday, no, Wednesday" -> "Book it for
//!    Wednesday"; "scratch that" drops what came before it.
//!
//! Everything is heuristic and errs on the side of leaving text alone: a
//! correction marker only fires when the words after it clearly restate or
//! replace the words before it.

#[derive(Debug, Clone, Copy)]
pub struct PolishOptions {
    pub remove_fillers: bool,
    pub voice_commands: bool,
    pub self_corrections: bool,
}

pub fn polish(text: &str, opts: PolishOptions) -> String {
    let mut paragraphs = parse(text);
    if opts.remove_fillers {
        paragraphs = paragraphs.into_iter().map(remove_fillers).collect();
    }
    if opts.voice_commands {
        paragraphs = apply_voice_commands(paragraphs);
        paragraphs = break_on_topic_shifts(paragraphs);
    }
    if opts.self_corrections {
        paragraphs = paragraphs.into_iter().map(apply_corrections).collect();
    }
    render(&paragraphs)
}

/// A run of words followed by `breaks` newlines (0 for the last paragraph).
#[derive(Debug, Clone, Default)]
struct Paragraph {
    words: Vec<String>,
    breaks: usize,
}

fn parse(text: &str) -> Vec<Paragraph> {
    let mut paragraphs = vec![Paragraph::default()];
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            let n = paragraphs.len();
            if paragraphs[n - 1].words.is_empty() && n > 1 {
                // Blank line: add to the previous paragraph's break count.
                paragraphs[n - 2].breaks += 1;
            } else {
                paragraphs[n - 1].breaks = 1;
                paragraphs.push(Paragraph::default());
            }
        }
        let last = paragraphs.last_mut().unwrap();
        last.words
            .extend(line.split_whitespace().map(str::to_string));
    }
    paragraphs
}

fn render(paragraphs: &[Paragraph]) -> String {
    let mut out = String::new();
    for p in paragraphs {
        out.push_str(&p.words.join(" "));
        for _ in 0..p.breaks {
            out.push('\n');
        }
    }
    out
}

// ---------------------------------------------------------------- word helpers

/// Lowercased word with surrounding punctuation stripped ("Um," -> "um").
fn core(word: &str) -> String {
    word.trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
        .replace('\u{2019}', "'")
}

fn trailing_punct(word: &str) -> &str {
    let end = word.trim_end_matches(|c: char| !c.is_alphanumeric()).len();
    &word[end..]
}

fn strip_trailing_punct(word: &str) -> &str {
    word.trim_end_matches(|c: char| !c.is_alphanumeric())
}

fn ends_sentence(word: &str) -> bool {
    trailing_punct(word).contains(['.', '?', '!'])
}

/// Ends with a clause separator: comma, dash, colon, ellipsis...
fn ends_clause(word: &str) -> bool {
    let t = trailing_punct(word);
    !t.is_empty() && (!ends_sentence(word) || t.contains("...") || t.contains('\u{2026}'))
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn starts_lowercase(word: &str) -> bool {
    word.chars()
        .find(|c| c.is_alphabetic())
        .is_some_and(char::is_lowercase)
}

// ------------------------------------------------------------------- fillers

const FILLERS: &[&str] = &[
    "um", "umm", "ummm", "uhm", "uh", "uhh", "uhhh", "er", "erm", "errm", "ah", "ahh", "hmm",
    "hmmm", "hm", "mm", "mmm", "mh", "eh", "ehh", "euh", "mm-hmm",
];

fn is_filler(word: &str) -> bool {
    FILLERS.contains(&core(word).as_str())
}

fn remove_fillers(p: Paragraph) -> Paragraph {
    let mut out: Vec<String> = Vec::with_capacity(p.words.len());
    let mut cap_next = false;
    let mut removed_last = false;

    for word in p.words {
        if is_filler(&word) {
            let at_sentence_start = out.last().is_none_or(|w| ends_sentence(w));
            let terminal: String = trailing_punct(&word)
                .chars()
                .filter(|c| matches!(c, '.' | '?' | '!'))
                .take(1)
                .collect();
            if !terminal.is_empty() && !at_sentence_start {
                // "I think, um." -> "I think."
                let prev = out.last_mut().unwrap();
                *prev = format!("{}{}", strip_trailing_punct(prev), terminal);
            }
            cap_next = at_sentence_start || !terminal.is_empty();
            removed_last = true;
            continue;
        }
        out.push(if cap_next && starts_lowercase(&word) {
            capitalize(&word)
        } else {
            word
        });
        cap_next = false;
        removed_last = false;
    }

    // "I think, um" -> "I think"
    if removed_last {
        if let Some(last) = out.last_mut() {
            if trailing_punct(last).ends_with(',') {
                *last = strip_trailing_punct(last).to_string();
            }
        }
    }

    Paragraph {
        words: out,
        breaks: p.breaks,
    }
}

// ------------------------------------------------------------ voice commands

/// Words that, right after "new line", mean the phrase is ordinary speech
/// ("a new line of products", "the new line is open").
const NOT_COMMAND_AFTER: &[&str] = &[
    "of", "for", "in", "on", "at", "to", "that", "which", "between", "is", "was", "are", "were",
    "from", "with", "item", "items", "manager", "managers", "will", "has", "had",
];
const NOT_COMMAND_BEFORE: &[&str] = &[
    "a", "an", "the", "this", "that", "our", "my", "your", "their", "his", "her", "its", "another",
    "every", "each", "any", "no", "one", "first", "second",
];

/// Returns (words consumed, newlines) if a voice command starts at `i`.
fn voice_command_at(words: &[String], i: usize) -> Option<(usize, usize)> {
    let w0 = core(&words[i]);
    let (len, breaks) = match w0.as_str() {
        "newline" => (1, 1),
        "new" | "next" => match words.get(i + 1).map(|w| core(w)).as_deref() {
            Some("line") if w0 == "new" => (2, 1),
            Some("paragraph") => (2, 2),
            _ => return None,
        },
        _ => return None,
    };
    let last = &words[i + len - 1];
    let punct_before = i == 0 || ends_sentence(&words[i - 1]) || ends_clause(&words[i - 1]);
    let punct_after = !trailing_punct(last).is_empty();
    if !punct_before && i > 0 && NOT_COMMAND_BEFORE.contains(&core(&words[i - 1]).as_str()) {
        return None;
    }
    if !punct_after {
        if let Some(next) = words.get(i + len) {
            if NOT_COMMAND_AFTER.contains(&core(next).as_str()) {
                return None;
            }
        }
    }
    Some((len, breaks))
}

fn apply_voice_commands(paragraphs: Vec<Paragraph>) -> Vec<Paragraph> {
    let mut out: Vec<Paragraph> = Vec::new();
    for p in paragraphs {
        let mut current = Paragraph::default();
        let mut cap_next = false;
        let mut i = 0;
        while i < p.words.len() {
            if let Some((len, breaks)) = voice_command_at(&p.words, i) {
                if current.words.is_empty() && !out.is_empty() {
                    // Command at the start of a paragraph: widen the previous break.
                    let prev = out.last_mut().unwrap();
                    prev.breaks = prev.breaks.max(breaks);
                } else if !current.words.is_empty() {
                    current.breaks = breaks;
                    out.push(std::mem::take(&mut current));
                }
                cap_next = true;
                i += len;
                continue;
            }
            let word = &p.words[i];
            current.words.push(if cap_next && starts_lowercase(word) {
                capitalize(word)
            } else {
                word.clone()
            });
            cap_next = false;
            i += 1;
        }
        if current.words.is_empty() && !out.is_empty() {
            let prev = out.last_mut().unwrap();
            prev.breaks = prev.breaks.max(p.breaks);
        } else {
            current.breaks = p.breaks;
            out.push(current);
        }
    }
    out
}

// ------------------------------------------------------------- topic shifts

/// Sentence openers that signal a change of subject: a new paragraph starts
/// there without the speaker having to say "new paragraph".
const TOPIC_SHIFTS: &[&[&str]] = &[
    &["also"],
    &["and", "also"],
    &["oh", "and"],
    &["anyway"],
    &["anyways"],
    &["separately"],
    &["additionally"],
    &["furthermore"],
    &["secondly"],
    &["thirdly"],
    &["finally"],
    &["lastly"],
    &["meanwhile"],
    &["regarding"],
    &["as", "for"],
    &["by", "the", "way"],
    &["moving", "on"],
    &["one", "more", "thing"],
    &["another", "thing"],
    &["in", "other", "news"],
    &["changing", "the", "subject"],
    &["other", "than", "that"],
    &["apart", "from", "that"],
    &["aside", "from", "that"],
    &["on", "another", "note"],
    &["on", "a", "different", "note"],
    &["on", "a", "separate", "note"],
    &["on", "a", "side", "note"],
    &["on", "a", "related", "note"],
];

/// Closing lines that get a paragraph of their own in a longer message.
const SIGN_OFFS: &[&[&str]] = &[
    &["thanks"],
    &["thank", "you"],
    &["many", "thanks"],
    &["cheers"],
    &["regards"],
    &["kind", "regards"],
    &["best", "regards"],
    &["best", "wishes"],
    &["speak", "soon"],
    &["talk", "soon"],
];

/// Sign-offs only split messages with at least this many sentences.
const SIGN_OFF_MIN_SENTENCES: usize = 3;
const SIGN_OFF_MAX_WORDS: usize = 4;

fn phrase_at(words: &[String], i: usize, phrases: &[&[&str]]) -> bool {
    phrases.iter().any(|phrase| {
        i + phrase.len() <= words.len()
            && words[i..i + phrase.len()]
                .iter()
                .zip(phrase.iter())
                .all(|(w, p)| core(w) == *p)
    })
}

fn break_on_topic_shifts(paragraphs: Vec<Paragraph>) -> Vec<Paragraph> {
    let sentences = paragraphs
        .iter()
        .flat_map(|p| p.words.iter())
        .filter(|w| ends_sentence(w))
        .count();
    let last_index = paragraphs.len().saturating_sub(1);
    let mut out = Vec::with_capacity(paragraphs.len());
    for (index, p) in paragraphs.into_iter().enumerate() {
        let n = p.words.len();
        let sign_off_at = (index == last_index && sentences >= SIGN_OFF_MIN_SENTENCES)
            .then(|| {
                (n.saturating_sub(SIGN_OFF_MAX_WORDS).max(1)..n)
                    .find(|&i| ends_sentence(&p.words[i - 1]) && phrase_at(&p.words, i, SIGN_OFFS))
            })
            .flatten();
        let mut current = Paragraph::default();
        for i in 0..n {
            let starts_new = !current.words.is_empty()
                && ends_sentence(&p.words[i - 1])
                && (sign_off_at == Some(i) || phrase_at(&p.words, i, TOPIC_SHIFTS));
            if starts_new {
                current.breaks = 2;
                out.push(std::mem::take(&mut current));
            }
            current.words.push(p.words[i].clone());
        }
        current.breaks = p.breaks;
        out.push(current);
    }
    out
}

// ---------------------------------------------------------- self-corrections

#[derive(Clone, Copy, PartialEq)]
enum Strength {
    /// Always a correction ("scratch that").
    Strong,
    /// Only a correction when what follows clearly restates what preceded.
    Weak,
    /// Weak, and must be set off by punctuation (", no, ") to count at all.
    WeakDelimited,
}

const MARKERS: &[(&[&str], Strength)] = &[
    (&["no", "scratch", "that"], Strength::Strong),
    (&["scratch", "that"], Strength::Strong),
    (&["strike", "that"], Strength::Strong),
    (&["delete", "that"], Strength::Strong),
    (&["forget", "that"], Strength::Strong),
    (&["sorry", "i", "mean"], Strength::Weak),
    (&["sorry", "i", "meant"], Strength::Weak),
    (&["no", "i", "mean"], Strength::Weak),
    (&["no", "wait"], Strength::Weak),
    (&["wait", "no"], Strength::Weak),
    (&["no", "sorry"], Strength::Weak),
    (&["or", "rather"], Strength::Weak),
    (&["i", "mean"], Strength::Weak),
    (&["i", "meant"], Strength::Weak),
    (&["actually"], Strength::Weak),
    (&["no"], Strength::WeakDelimited),
    (&["sorry"], Strength::WeakDelimited),
    (&["wait"], Strength::WeakDelimited),
    (&["rather"], Strength::WeakDelimited),
    (&["correction"], Strength::WeakDelimited),
];

/// Too common to anchor a one-word restatement on.
const FUNCTION_WORDS: &[&str] = &[
    "a", "an", "the", "to", "of", "in", "on", "at", "for", "and", "or", "but", "so", "is", "it",
    "i", "i'm", "i'll", "i'd", "i've", "we", "you", "he", "she", "they", "it's", "that", "this",
    "be", "was", "are", "my", "me", "with", "just", "then",
];

/// Words that open a clause: "That's true, actually it's not" restates the clause.
const SUBJECTS: &[&str] = &[
    "i", "i'm", "i'll", "i'd", "i've", "we", "we're", "we'll", "we'd", "we've", "you", "you're",
    "you'll", "you'd", "he", "he's", "he'll", "she", "she's", "she'll", "they", "they're",
    "they'll", "it", "it's", "it'll", "that", "that's", "this", "there", "there's", "let's",
];

/// "..., actually, if that's ok" is not a replacement.
const CONTINUATIONS: &[&str] = &[
    "if", "but", "and", "so", "because", "when", "though", "although", "since", "unless", "which",
    "that", "as", "too", "also", "anyway", "though",
];

const NUMBER_WORDS: &[&str] = &[
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
    "twenty",
    "thirty",
    "forty",
    "fifty",
    "sixty",
    "seventy",
    "eighty",
    "ninety",
    "hundred",
    "thousand",
    "half",
];

fn is_number(core: &str) -> bool {
    NUMBER_WORDS.contains(&core)
        || (core.chars().any(|c| c.is_ascii_digit())
            && core
                .chars()
                .all(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | ':')))
        || ["st", "nd", "rd", "th"].iter().any(|s| {
            core.strip_suffix(s)
                .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        })
}

fn match_marker(cores: &[String], i: usize) -> Option<(usize, Strength)> {
    MARKERS.iter().find_map(|(words, strength)| {
        let end = i + words.len();
        (end <= cores.len() && cores[i..end].iter().zip(words.iter()).all(|(a, b)| a == b))
            .then_some((words.len(), *strength))
    })
}

fn sentence_start(words: &[String], before: usize) -> usize {
    (0..before)
        .rev()
        .find(|&j| ends_sentence(&words[j]))
        .map_or(0, |j| j + 1)
}

fn sentence_end(words: &[String], from: usize) -> usize {
    (from..words.len())
        .find(|&j| ends_sentence(&words[j]))
        .map_or(words.len(), |j| j + 1)
}

/// An unpunctuated marker needs at least this many restated words.
const UNDELIMITED_RESTATEMENT_WORDS: usize = 3;

/// Where the replacement `a` restates part of the corrected words `b`:
/// (offset in b, matched length), preferring the longest match.
fn restatement(b: &[String], a: &[String]) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize)> = None;
    for i in (0..b.len()).rev() {
        let k = b[i..]
            .iter()
            .zip(a.iter())
            .take_while(|(x, y)| x == y)
            .count();
        let strong_enough = k >= 2 || (k == 1 && !FUNCTION_WORDS.contains(&a[0].as_str()));
        if strong_enough && best.is_none_or(|(_, bk)| k > bk) {
            best = Some((i, k));
        }
    }
    best
}

// --------------------------------------------------------------- false starts

/// Longest abandoned tail allowed after the repeated opening words.
const FALSE_START_MAX_ABANDONED: usize = 2;
/// Opening words that must be repeated for a restart to count.
const FALSE_START_MIN_REPEAT: usize = 2;
const FALSE_START_MAX_REPEAT: usize = 4;

/// Trailing punctuation that marks a cut-off word ("temper...", "temper-").
fn is_truncated(word: &str) -> bool {
    let t = trailing_punct(word);
    t.contains("...") || t.contains(['\u{2026}', '-', '\u{2013}', '\u{2014}'])
}

/// Finds a restart: the speaker began a phrase, gave up, and said it again
/// ("What's the temper... what's the weather"). Returns (start, restart).
fn find_false_start(words: &[String], cores: &[String]) -> Option<(usize, usize)> {
    for r in 1..words.len() {
        let boundary = &words[r - 1];
        for k in (FALSE_START_MIN_REPEAT..=FALSE_START_MAX_REPEAT).rev() {
            if r + k >= words.len() {
                continue; // the restart must carry on past the repeated words
            }
            for abandoned in 0..=FALSE_START_MAX_ABANDONED {
                let Some(s) = r.checked_sub(k + abandoned) else {
                    continue;
                };
                let repeated = &cores[r..r + k];
                if cores[s..s + k] != *repeated
                    || repeated
                        .iter()
                        .all(|w| FUNCTION_WORDS.contains(&w.as_str()))
                    || words[s..r - 1]
                        .iter()
                        .any(|w| !trailing_punct(w).is_empty())
                {
                    continue;
                }
                // "I'll check and I'll call you" joins two clauses; it is not a restart.
                if cores[s + k..r].iter().any(|w| {
                    CONTINUATIONS.contains(&w.as_str()) || matches!(w.as_str(), "or" | "then")
                }) {
                    continue;
                }
                // A full stop is not enough: "I love it. I love it so much." is real speech.
                let ok = is_truncated(boundary) || trailing_punct(boundary).is_empty();
                if ok {
                    return Some((s, r));
                }
            }
        }
    }
    None
}

fn remove_false_starts(mut words: Vec<String>) -> Vec<String> {
    for _ in 0..16 {
        let cores: Vec<String> = words.iter().map(|w| core(w)).collect();
        let Some((s, r)) = find_false_start(&words, &cores) else {
            break;
        };
        let sentence_start = s == 0 || ends_sentence(&words[s - 1]);
        let mut restart = words.split_off(r);
        words.truncate(s);
        if sentence_start && starts_lowercase(&restart[0]) {
            restart[0] = capitalize(&restart[0]);
        }
        words.extend(restart);
    }
    words
}

/// Rewrites one correction. Returns the new word list, or None if the marker
/// at `m` should be left alone.
fn try_correction(words: &[String], cores: &[String], m: usize) -> Option<Vec<String>> {
    let (len, strength) = match_marker(cores, m)?;
    let e = m + len;
    let marker_last = &words[e - 1];
    let marker_delimited = !trailing_punct(marker_last).is_empty();

    // "three no four pallets" / "at 3 no 4" -> swap just the number.
    if cores[m] == "no"
        && len == 1
        && m > 0
        && e < words.len()
        && is_number(&cores[m - 1])
        && is_number(&cores[e])
        && !ends_sentence(&words[m - 1])
    {
        let mut out = words[..m - 1].to_vec();
        out.extend_from_slice(&words[e..]);
        return Some(out);
    }

    let at_start = m == 0 || ends_sentence(&words[m - 1]);
    let mid = !at_start && ends_clause(&words[m - 1]);
    let mid = mid || (!at_start && strength == Strength::Strong);
    if !at_start && !mid {
        // No punctuation around the marker: only a clear restatement counts.
        // "I want to go to the moon actually I want to go to Mars" -> "I want
        // to go to Mars", but "I actually like it" stays.
        let s0 = sentence_start(words, m);
        let a_end = sentence_end(words, e);
        let (keep, _) = restatement(&cores[s0..m], &cores[e..a_end])
            .filter(|&(_, k)| k >= UNDELIMITED_RESTATEMENT_WORDS)?;
        let mut out = words[..s0 + keep].to_vec();
        for (k, w) in words[e..].iter().enumerate() {
            out.push(if k == 0 && keep == 0 && starts_lowercase(w) {
                capitalize(w)
            } else {
                w.clone()
            });
        }
        return Some(out);
    }
    if strength == Strength::WeakDelimited && (!marker_delimited || at_start) {
        return None; // "no problem", "Sorry, I'm late."
    }
    if at_start && strength != Strength::Strong && !marker_delimited {
        return None; // "I mean what I say."
    }

    // B: the words being corrected. A: the replacement.
    let s0 = if at_start {
        if m == 0 {
            return None;
        }
        sentence_start(words, m - 1)
    } else {
        sentence_start(words, m)
    };
    let a_end = sentence_end(words, e);
    let b = &cores[s0..m];
    let a_words = &words[e..a_end];
    let a = &cores[e..a_end];
    if b.is_empty() {
        return None;
    }

    let rebuild = |keep_until: usize, capitalize_first: bool| -> Vec<String> {
        let mut out: Vec<String> = words[..keep_until].to_vec();
        for (k, w) in a_words.iter().enumerate() {
            out.push(if k == 0 && capitalize_first && starts_lowercase(w) {
                capitalize(w)
            } else {
                w.clone()
            });
        }
        out.extend_from_slice(&words[a_end..]);
        out
    };

    if a.is_empty() {
        if strength != Strength::Strong {
            return None; // "It was fine, actually."
        }
        // "Let's go to Tesco. Scratch that." -> drop the sentence.
        return Some(rebuild(s0, false));
    }

    // 1. Restatement: the replacement repeats the start of the corrected phrase.
    if let Some((i, _)) = restatement(b, a) {
        return Some(rebuild(s0 + i, i == 0));
    }

    if at_start {
        // Across a sentence boundary only a clear restatement or a strong
        // marker counts ("Is it Tuesday? No, it's Wednesday." stays).
        return (strength == Strength::Strong).then(|| rebuild(s0, true));
    }

    let a0 = a[0].as_str();
    if CONTINUATIONS.contains(&a0) {
        return None;
    }

    // 2. Clause restatement: "That's true, actually it's not" -> "It's not".
    if SUBJECTS.contains(&a0) {
        if SUBJECTS.contains(&b[0].as_str()) && a.len() <= 6 || strength == Strength::Strong {
            return Some(rebuild(s0, true));
        }
        return None;
    }

    // 3. Short swap: "Tuesday, no, Wednesday", "50 pounds, actually 60 pounds".
    if a.len() <= 3 {
        let n = if b.len() == 1 {
            1
        } else {
            a.len().min(b.len() - 1)
        };
        let keep = m - n;
        return Some(rebuild(keep, keep == s0));
    }

    (strength == Strength::Strong).then(|| rebuild(s0, true))
}

fn apply_corrections(p: Paragraph) -> Paragraph {
    let mut words = remove_false_starts(p.words);
    let mut m = 0;
    let mut guard = 0;
    while m < words.len() && guard < 64 {
        guard += 1;
        let cores: Vec<String> = words.iter().map(|w| core(w)).collect();
        match try_correction(&words, &cores, m) {
            Some(next) => {
                words = next;
                m = 0;
            }
            None => m += 1,
        }
    }
    Paragraph {
        words,
        breaks: p.breaks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(text: &str) -> String {
        polish(
            text,
            PolishOptions {
                remove_fillers: true,
                voice_commands: true,
                self_corrections: true,
            },
        )
    }

    // fillers
    #[test]
    fn filler_mid_sentence_with_commas() {
        assert_eq!(
            all("So, um, I think we should go."),
            "So, I think we should go."
        );
    }
    #[test]
    fn filler_at_start_capitalises_next() {
        assert_eq!(all("Um, so I think it's fine."), "So I think it's fine.");
        assert_eq!(all("uh yes please"), "Yes please");
    }
    #[test]
    fn filler_carrying_full_stop() {
        assert_eq!(all("I think, um. Yes."), "I think. Yes.");
    }
    #[test]
    fn filler_at_end_drops_dangling_comma() {
        assert_eq!(all("I think, uh"), "I think");
    }
    #[test]
    fn filler_plain_words() {
        assert_eq!(all("I um think er we can"), "I think we can");
    }
    #[test]
    fn filler_not_in_real_words() {
        assert_eq!(all("The hammer is here."), "The hammer is here.");
    }
    #[test]
    fn fillers_kept_when_disabled() {
        let opts = PolishOptions {
            remove_fillers: false,
            voice_commands: true,
            self_corrections: true,
        };
        assert_eq!(polish("I um think", opts), "I um think");
    }

    // voice commands
    #[test]
    fn new_line_with_punctuation() {
        assert_eq!(
            all("Hi John, new line, how are you?"),
            "Hi John,\nHow are you?"
        );
        assert_eq!(all("Thanks. New line. Calvin."), "Thanks.\nCalvin.");
    }
    #[test]
    fn new_line_without_punctuation() {
        assert_eq!(all("Hi John new line how are you"), "Hi John\nHow are you");
    }
    #[test]
    fn new_paragraph() {
        assert_eq!(
            all("That's all. New paragraph. Next topic."),
            "That's all.\n\nNext topic."
        );
        assert_eq!(all("Done next paragraph more"), "Done\n\nMore");
    }
    #[test]
    fn new_line_at_end_and_start() {
        assert_eq!(all("Hello. New line."), "Hello.\n");
        assert_eq!(all("New line. Hello."), "Hello.");
    }
    #[test]
    fn new_line_as_ordinary_speech() {
        assert_eq!(
            all("We launched a new line of products."),
            "We launched a new line of products."
        );
        assert_eq!(all("The new line is open."), "The new line is open.");
    }

    // self-corrections
    #[test]
    fn correction_restatement() {
        assert_eq!(
            all("I want an apple, actually I want a pear."),
            "I want a pear."
        );
        assert_eq!(
            all("i want an apple, actually i want a pear"),
            "I want a pear"
        );
    }
    #[test]
    fn correction_restatement_mid_sentence() {
        assert_eq!(
            all("Send it to John, actually send it to Mary."),
            "Send it to Mary."
        );
        assert_eq!(
            all("Please book the room and send it to John, I mean send it to Mary."),
            "Please book the room and send it to Mary."
        );
    }
    #[test]
    fn correction_across_sentences() {
        assert_eq!(
            all("I want an apple. Actually, I want a pear."),
            "I want a pear."
        );
    }
    #[test]
    fn correction_short_swap() {
        assert_eq!(
            all("Book it for Tuesday, no, Wednesday."),
            "Book it for Wednesday."
        );
        assert_eq!(
            all("It costs 50 pounds, actually 60 pounds."),
            "It costs 60 pounds."
        );
        assert_eq!(
            all("Meet at the cafe, actually the park."),
            "Meet at the park."
        );
        assert_eq!(all("Sounds good, actually great."), "Sounds great.");
        assert_eq!(all("Let's meet at 3, sorry, 4."), "Let's meet at 4.");
    }
    #[test]
    fn correction_number_swap_without_commas() {
        assert_eq!(
            all("So we need to order three no four pallets for the Leeds job."),
            "So we need to order four pallets for the Leeds job."
        );
        assert_eq!(all("Meet at 3 no 4 tomorrow."), "Meet at 4 tomorrow.");
        assert_eq!(
            all("Push it to the 20th, or rather the 21st."),
            "Push it to the 21st."
        );
        assert_eq!(
            all("I said no to two of them."),
            "I said no to two of them."
        );
    }
    #[test]
    fn correction_clause() {
        assert_eq!(all("That's true, actually it's not."), "It's not.");
        assert_eq!(all("I'm fine, actually I'm great."), "I'm great.");
    }
    #[test]
    fn correction_scratch_that() {
        assert_eq!(
            all("Let's meet Tuesday. Scratch that, let's meet Wednesday."),
            "Let's meet Wednesday."
        );
        assert_eq!(all("Hello there. Scratch that."), "");
        assert_eq!(
            all("Call Dave. Go to Tesco scratch that Asda."),
            "Call Dave. Go to Asda."
        );
    }
    #[test]
    fn correction_keeps_previous_sentences() {
        assert_eq!(
            all("Hi Sam. I want an apple, actually a pear."),
            "Hi Sam. I want a pear."
        );
    }
    #[test]
    fn non_corrections_left_alone() {
        for s in [
            "I actually like it.",
            "It was fine, actually.",
            "The meeting went well, actually the client loved it.",
            "Is it Tuesday? No, it's Wednesday.",
            "Sorry, I'm late.",
            "No problem at all.",
            "I mean what I say.",
            "I'll call you tomorrow, actually, if that's ok.",
            "That's great. Actually, I was wondering about the invoice.",
            "I would rather not.",
        ] {
            assert_eq!(all(s), s, "changed: {s}");
        }
    }
    #[test]
    fn corrections_kept_when_disabled() {
        let opts = PolishOptions {
            remove_fillers: true,
            voice_commands: true,
            self_corrections: false,
        };
        let s = "I want an apple, actually I want a pear.";
        assert_eq!(polish(s, opts), s);
    }
    #[test]
    fn everything_together() {
        assert_eq!(
            all(
                "Hi Sam, new line, um, I'll send it Tuesday, no, Wednesday. New paragraph. Thanks."
            ),
            "Hi Sam,\nI'll send it Wednesday.\n\nThanks."
        );
    }
}
#[cfg(test)]
mod smart_tests {
    use super::*;

    fn all(text: &str) -> String {
        polish(
            text,
            PolishOptions {
                remove_fillers: true,
                voice_commands: true,
                self_corrections: true,
            },
        )
    }

    #[test]
    fn restatement_without_punctuation() {
        for s in [
            "I want to go to the moon, actually I want to go to Mars.",
            "I want to go to the moon actually I want to go to Mars.",
            "I want to go to the moon no I want to go to Mars.",
        ] {
            assert_eq!(all(s), "I want to go to Mars.", "input: {s}");
        }
    }

    #[test]
    fn false_starts_are_dropped() {
        for s in [
            "What's the temper... What's the weather in LA right now?",
            "What's the temper what's the weather in LA right now?",
            "What's the temper- what's the weather in LA right now?",
        ] {
            assert_eq!(all(s), "What's the weather in LA right now?", "input: {s}");
        }
        assert_eq!(
            all("Can you tell me what's the temper... what's the weather in LA?"),
            "Can you tell me what's the weather in LA?"
        );
    }

    #[test]
    fn topic_shifts_start_paragraphs() {
        assert_eq!(
            all("I submitted the forms. They have not pulled through. Also, can you check the PDF? Thanks."),
            "I submitted the forms. They have not pulled through.

Also, can you check the PDF?

Thanks."
        );
        assert_eq!(
            all("Hi John, the quote is attached. By the way, the install is moved to Friday. Thanks, Calvin."),
            "Hi John, the quote is attached.

By the way, the install is moved to Friday.

Thanks, Calvin."
        );
    }

    #[test]
    fn short_messages_keep_sign_off_inline() {
        assert_eq!(
            all("Can you check it? Thanks."),
            "Can you check it? Thanks."
        );
    }

    #[test]
    fn ordinary_speech_left_alone() {
        for s in [
            "Is it red, is it blue?",
            "What's the plan, what's the budget?",
            "I think that that is fine.",
            "It's going to rain. It's going to rain all week.",
            "I love it. I love it so much.",
            "We need to go to the shop, to the bank and to the post office.",
            "I also want to know.",
            "Finally we got it working.",
            "I will go there and I will come back later.",
            "I'll check and I'll call you.",
            "We can go but we can't stay.",
        ] {
            assert_eq!(all(s), s, "changed: {s}");
        }
    }
}
