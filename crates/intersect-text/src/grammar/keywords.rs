//! Deterministic punctuation keyword matching for the native Xtext lexer.
//! Arena storage avoids recursive construction/destruction for long keywords.
use std::collections::BTreeMap;

/// The current increment admits punctuation-only keywords without hidden-token
/// delimiters. Mixed word/punctuation and terminal-rule lexing remain explicit gaps.
pub fn is_punctuation_keyword(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_punctuation() && !b"'\"_".contains(&byte))
        && !text.contains("//")
        && !text.contains("/*")
}

#[derive(Default)]
struct State {
    edges: BTreeMap<u8, usize>,
    accepting: bool,
}

pub struct PunctuationKeywords {
    states: Vec<State>,
}
impl PunctuationKeywords {
    pub fn new<'a>(keywords: impl IntoIterator<Item = &'a str>) -> Self {
        Self::all(
            keywords
                .into_iter()
                .filter(|text| is_punctuation_keyword(text)),
        )
    }

    /// Complete literal matching for explicit grammar-defined lexical policy.
    pub fn all<'a>(keywords: impl IntoIterator<Item = &'a str>) -> Self {
        let mut states = vec![State::default()];
        for keyword in keywords {
            let mut state = 0;
            for byte in keyword.bytes() {
                let next = if let Some(next) = states[state].edges.get(&byte) {
                    *next
                } else {
                    let next = states.len();
                    states.push(State::default());
                    states[state].edges.insert(byte, next);
                    next
                };
                state = next;
            }
            states[state].accepting = true;
        }
        Self { states }
    }

    /// Length in bytes of the longest complete keyword at this source offset.
    /// No character copying, trivia skipping or fallback parser choices occur here.
    pub fn match_len(&self, input: &str) -> Option<usize> {
        let mut state = 0;
        let mut accepted = None;
        for (index, byte) in input.bytes().enumerate() {
            let Some(next) = self.states[state].edges.get(&byte) else {
                break;
            };
            state = *next;
            if self.states[state].accepting {
                accepted = Some(index + 1);
            }
        }
        accepted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn longest_complete_prefix_is_independent_of_insertion_order() {
        let words = [":", "::", ":>", ":>>", "*", "**", "::**"];
        let forward = PunctuationKeywords::new(words);
        let reverse = PunctuationKeywords::new(words.into_iter().rev());
        for (input, expected) in [
            ("::**;", Some(4)),
            ("::x", Some(2)),
            (":>>Wheel", Some(3)),
            (":> Wheel", Some(2)),
            (": Wheel", Some(1)),
            ("***", Some(2)),
            ("word", None),
            ("", None),
        ] {
            assert_eq!(forward.match_len(input), expected, "{input}");
            assert_eq!(reverse.match_len(input), expected, "{input}");
        }
    }
    #[test]
    fn hidden_trivia_and_incomplete_edges_never_join_keyword_bytes() {
        let matcher = PunctuationKeywords::new(["::", "**", ":>>"]);
        for text in [
            ":",
            ":>",
            ": :",
            ":/*gap*/:",
            "://gap\n:",
            "* *",
            " ::",
            "\"::\"",
        ] {
            assert_eq!(matcher.match_len(text), None, "{text}");
        }
        assert_eq!(matcher.match_len("::é"), Some(2));
        assert_eq!(matcher.match_len("::**"), Some(2));
    }
    #[test]
    fn unsupported_keyword_shapes_are_explicit_and_not_inserted() {
        for text in [
            "", "name", "a-b", " ", "_", "'", "\"", "/*", "//", "::/*", "é::",
        ] {
            assert!(!is_punctuation_keyword(text), "{text}");
            assert_eq!(
                PunctuationKeywords::new([text]).match_len(text),
                None,
                "{text}"
            );
        }
        for text in ["::", "**", ":>>", "/", "/=", "*/", "->", "{", ";"] {
            assert!(is_punctuation_keyword(text), "{text}");
        }
    }
    #[test]
    fn very_long_keyword_uses_iterative_arena_storage() {
        let word = ":".repeat(50_000);
        let matcher = PunctuationKeywords::new([word.as_str()]);
        assert_eq!(matcher.match_len(&word), Some(word.len()));
        assert_eq!(matcher.match_len(&word[..word.len() - 1]), None);
        // Dropping the arena must not recurse once per source byte.
        drop(matcher);
    }
}
