use std::iter::Peekable;
use std::mem;
use std::str::Chars;

use thiserror::Error;

/// Longest text accepted for one read-aloud request, in Unicode scalar values after trimming.
pub const MAX_READING_CHARS: usize = 20_000;

/// Longest piece sent to the synthesizer at once. Longer sentences are split between words.
pub const MAX_SENTENCE_CHARS: usize = 300;

const ELLIPSIS: char = '\u{2026}';

/// Why a raw string is not a valid [`ReadingText`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TextError {
    /// Nothing left after trimming whitespace.
    #[error("there is no text to read")]
    Empty,
    /// Longer than [`MAX_READING_CHARS`] after trimming.
    #[error(
        "the text is {length} characters; the most that can be read at once is {MAX_READING_CHARS}"
    )]
    TooLong {
        /// Character count of the trimmed input.
        length: usize,
    },
}

/// One piece of a [`ReadingText`] as it is spoken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sentence {
    /// The words of the sentence, trimmed.
    pub text: String,
    /// Whether it opens a paragraph: it is the first sentence, or follows a line break.
    pub starts_paragraph: bool,
}

/// Trimmed text of 1 to [`MAX_READING_CHARS`] characters, ready to be read aloud.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadingText(String);

impl ReadingText {
    /// Trims `raw` and validates its length.
    ///
    /// # Errors
    ///
    /// [`TextError::Empty`] if only whitespace remains, [`TextError::TooLong`] if more than
    /// [`MAX_READING_CHARS`] characters remain.
    pub fn parse(raw: &str) -> Result<Self, TextError> {
        let trimmed = raw.trim();
        let length = trimmed.chars().count();
        if length == 0 {
            return Err(TextError::Empty);
        }
        if length > MAX_READING_CHARS {
            return Err(TextError::TooLong { length });
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// The validated text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Number of characters (Unicode scalar values).
    #[must_use]
    pub fn char_count(&self) -> usize {
        self.0.chars().count()
    }

    /// The first `max_chars` characters with whitespace collapsed, ending in an ellipsis when cut.
    #[must_use]
    pub fn preview(&self, max_chars: usize) -> String {
        let collapsed = self.0.split_whitespace().collect::<Vec<_>>().join(" ");
        if collapsed.chars().count() <= max_chars {
            return collapsed;
        }
        let mut cut: String = collapsed.chars().take(max_chars).collect();
        cut.truncate(cut.trim_end().len());
        cut.push(ELLIPSIS);
        cut
    }

    /// The text as pieces of at most [`MAX_SENTENCE_CHARS`] characters, in reading order.
    ///
    /// Splits after `.`, `!`, `?`, or `…` followed by whitespace, and at every line break,
    /// so the first sentence can be spoken while the rest is still being synthesized.
    #[must_use]
    pub fn sentences(&self) -> Vec<String> {
        self.split()
            .into_iter()
            .map(|sentence| sentence.text)
            .collect()
    }

    /// Whether `sentence` has anything to say: a letter or a digit. A Markdown table rule
    /// such as `| --- | --- |` does not, and voice servers fail on text that gives no audio.
    #[must_use]
    pub fn is_speakable(sentence: &str) -> bool {
        sentence.chars().any(char::is_alphanumeric)
    }

    /// The same pieces as [`Self::sentences`], each marked with whether it opens a paragraph.
    #[must_use]
    pub fn split(&self) -> Vec<Sentence> {
        let mut sentences = Vec::new();
        let mut current = String::new();
        let mut after_break = true;
        let mut chars = self.0.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '\n' {
                current.push(c);
            }
            if ends_sentence(c, &mut chars) {
                after_break = flush(&mut sentences, &mut current, after_break);
            }
            if c == '\n' {
                after_break = true;
            }
        }
        flush(&mut sentences, &mut current, after_break);
        sentences.into_iter().flat_map(split_long).collect()
    }
}

fn ends_sentence(c: char, rest: &mut Peekable<Chars<'_>>) -> bool {
    let terminator = matches!(c, '.' | '!' | '?' | ELLIPSIS);
    c == '\n' || (terminator && rest.peek().is_none_or(|next| next.is_whitespace()))
}

/// Pushes the sentence in `current`, if any. Returns whether the next one still opens a
/// paragraph: true only when nothing was pushed and this one would have.
fn flush(sentences: &mut Vec<Sentence>, current: &mut String, starts_paragraph: bool) -> bool {
    let sentence = mem::take(current);
    let trimmed = sentence.trim();
    if trimmed.is_empty() {
        return starts_paragraph;
    }
    sentences.push(Sentence {
        text: trimmed.to_owned(),
        starts_paragraph,
    });
    false
}

fn split_long(sentence: Sentence) -> Vec<Sentence> {
    let Sentence {
        text,
        starts_paragraph,
    } = sentence;
    wrap(text)
        .into_iter()
        .enumerate()
        .map(|(index, words)| Sentence {
            text: words,
            starts_paragraph: starts_paragraph && index == 0,
        })
        .collect()
}

fn wrap(text: String) -> Vec<String> {
    if text.chars().count() <= MAX_SENTENCE_CHARS {
        return vec![text];
    }
    let mut pieces = Vec::new();
    let mut piece = String::new();
    for word in text.split_whitespace() {
        let needed = piece.chars().count() + 1 + word.chars().count();
        if !piece.is_empty() && needed > MAX_SENTENCE_CHARS {
            pieces.push(mem::take(&mut piece));
        }
        if !piece.is_empty() {
            piece.push(' ');
        }
        piece.push_str(word);
    }
    pieces.push(piece);
    pieces
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sentences(raw: &str) -> Vec<String> {
        ReadingText::parse(raw).unwrap().sentences()
    }

    #[test]
    fn a_piece_may_fill_exactly_the_limit() {
        let first = "a".repeat(149);
        let second = "b".repeat(150);
        assert_eq!(
            sentences(&format!("{first} {second} c")),
            [format!("{first} {second}"), "c".to_owned()]
        );
    }

    #[test]
    fn a_word_that_would_pass_the_limit_starts_a_new_piece() {
        let first = "a".repeat(149);
        let second = "b".repeat(151);
        assert_eq!(sentences(&format!("{first} {second}")), [first, second]);
    }

    #[test]
    fn trims_and_rejects_empty_text() {
        assert_eq!(ReadingText::parse("  hi \n").unwrap().as_str(), "hi");
        assert_eq!(ReadingText::parse(" \n\t "), Err(TextError::Empty));
    }

    #[test]
    fn accepts_exactly_the_limit_and_rejects_one_more() {
        let at_limit = "a".repeat(MAX_READING_CHARS);
        assert!(ReadingText::parse(&at_limit).is_ok());
        let over = "é".repeat(MAX_READING_CHARS + 1);
        assert_eq!(
            ReadingText::parse(&over),
            Err(TextError::TooLong {
                length: MAX_READING_CHARS + 1
            })
        );
    }

    #[test]
    fn counts_characters_not_bytes() {
        assert_eq!(ReadingText::parse("héllo").unwrap().char_count(), 5);
    }

    #[test]
    fn only_text_with_a_letter_or_digit_is_speakable() {
        let speakable =
            ["Hi.", "**1.**", "é", "| --- | --- |", "- * .", ""].map(ReadingText::is_speakable);
        assert_eq!(
            speakable,
            [true, true, true, false, false, false],
            "{speakable:?}"
        );
    }

    #[test]
    fn error_messages_are_readable() {
        assert_eq!(TextError::Empty.to_string(), "there is no text to read");
        assert_eq!(
            TextError::TooLong { length: 20_001 }.to_string(),
            "the text is 20001 characters; the most that can be read at once is 20000"
        );
    }

    #[test]
    fn preview_keeps_short_text_and_collapses_whitespace() {
        let text = ReadingText::parse("Hello\n\n  world").unwrap();
        assert_eq!(text.preview(20), "Hello world");
        assert_eq!(text.preview(11), "Hello world");
    }

    #[test]
    fn preview_cuts_long_text_with_an_ellipsis_and_no_trailing_space() {
        let text = ReadingText::parse("Hello brave new world").unwrap();
        assert_eq!(text.preview(6), "Hello\u{2026}");
        assert_eq!(text.preview(8), "Hello br\u{2026}");
    }

    #[test]
    fn splits_after_terminal_punctuation_followed_by_space() {
        assert_eq!(
            sentences("One. Two! Three? Four\u{2026} Five"),
            ["One.", "Two!", "Three?", "Four\u{2026}", "Five"]
        );
    }

    #[test]
    fn keeps_decimals_and_dotted_names_together() {
        assert_eq!(sentences("Pi is 3.14 today."), ["Pi is 3.14 today."]);
        assert_eq!(sentences("See example.com now"), ["See example.com now"]);
    }

    #[test]
    fn splits_at_line_breaks_and_drops_blank_lines() {
        assert_eq!(sentences("Title\n\nBody text"), ["Title", "Body text"]);
    }

    #[test]
    fn splits_long_sentences_between_words_within_the_limit() {
        let word = "abcd";
        let long = vec![word; 100].join(" ");
        let pieces = sentences(&long);
        assert!(pieces.len() > 1, "a 499-character sentence must be split");
        assert!(
            pieces
                .iter()
                .all(|piece| piece.chars().count() <= MAX_SENTENCE_CHARS),
            "every piece fits the limit"
        );
        assert_eq!(pieces.join(" "), long);
    }

    #[test]
    fn a_sentence_exactly_at_the_limit_is_not_split() {
        let exact = "a".repeat(MAX_SENTENCE_CHARS);
        assert_eq!(sentences(&exact), [exact]);
    }

    fn paragraph_starts(raw: &str) -> Vec<(String, bool)> {
        ReadingText::parse(raw)
            .unwrap()
            .split()
            .into_iter()
            .map(|sentence| (sentence.text, sentence.starts_paragraph))
            .collect()
    }

    #[test]
    fn the_first_sentence_and_those_after_a_line_break_open_paragraphs() {
        assert_eq!(
            paragraph_starts("Good morning.\nTwo emails need you. Lunch is at noon."),
            [
                ("Good morning.".to_owned(), true),
                ("Two emails need you.".to_owned(), true),
                ("Lunch is at noon.".to_owned(), false),
            ]
        );
    }

    #[test]
    fn blank_lines_and_carriage_returns_open_one_paragraph() {
        assert_eq!(
            paragraph_starts("Title\r\n\r\nBody. More"),
            [
                ("Title".to_owned(), true),
                ("Body.".to_owned(), true),
                ("More".to_owned(), false),
            ]
        );
    }

    #[test]
    fn only_the_first_piece_of_a_long_sentence_opens_its_paragraph() {
        let first = "a".repeat(200);
        let second = "b".repeat(200);
        assert_eq!(
            paragraph_starts(&format!("Hi.\n{first} {second}")),
            [("Hi.".to_owned(), true), (first, true), (second, false)]
        );
    }

    #[test]
    fn a_single_word_longer_than_the_limit_stays_whole() {
        let word = "x".repeat(MAX_SENTENCE_CHARS + 5);
        assert_eq!(sentences(&format!("{word} end")), [word, "end".to_owned()]);
    }
}
