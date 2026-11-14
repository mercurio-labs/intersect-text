//! Handwritten cursor over immutable, source-bearing lexical winners.
//! The language owns policy selection; this module only traverses records.
use super::Token;

/// A parsing mark includes significant progress as well as raw traversal.
/// Skipping trivia must not create a match or extend a semantic source span.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TokenCursor {
    pub position: usize,
    pub last_matched: Option<usize>,
    pub matched: usize,
}

impl TokenCursor {
    /// Consume the currently selected significant token. Recognition remains
    /// with the existing parser; this operation cannot choose another lexer.
    pub fn consume(&mut self) -> Result<usize, String> {
        let selected = self.position;
        let next = self
            .position
            .checked_add(1)
            .ok_or("Token cursor position overflow")?;
        let matched = self
            .matched
            .checked_add(1)
            .ok_or("Token cursor progress overflow")?;
        self.position = next;
        self.last_matched = Some(selected);
        self.matched = matched;
        Ok(selected)
    }

    /// Observe LT(1), charging each visited raw record before changing position.
    /// On failure the denied record remains unconsumed. Marks are plain copies;
    /// token storage and source identities are never copied by this cursor.
    pub fn skip_hidden<T>(
        &mut self,
        tokens: &[Token<T>],
        mut hidden: impl FnMut(&Token<T>) -> Result<bool, String>,
        work: &mut Option<usize>,
    ) -> Result<(), String> {
        while let Some(token) = tokens.get(self.position) {
            if let Some(remaining) = work {
                if *remaining == 0 {
                    return Err("Retained token traversal budget exceeded".into());
                }
                *remaining -= 1;
            }
            if !hidden(token)? {
                break;
            }
            self.position += 1;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens() -> Vec<Token<&'static str>> {
        [" ", "a", "/*x*/", "b", " "]
            .into_iter()
            .enumerate()
            .map(|(index, text)| Token {
                text,
                start: index,
                end: index + 1,
                quoted: false,
                lexical_alternative: Some(index),
            })
            .collect()
    }

    #[test]
    fn raw_skip_does_not_publish_match_and_mark_rewinds_all_state() -> Result<(), String> {
        let tokens = tokens();
        let mut cursor = TokenCursor::default();
        let mark = cursor;
        let mut work = Some(6);
        cursor.skip_hidden(&tokens, |token| Ok(token.text == " "), &mut work)?;
        assert_eq!(cursor.position, 1);
        assert_eq!(cursor.last_matched, None);
        assert_eq!(cursor.matched, 0);
        assert_eq!(cursor.consume()?, 1);
        cursor.skip_hidden(&tokens, |token| Ok(token.text == "/*x*/"), &mut work)?;
        assert_eq!(cursor.position, 3);
        assert_eq!(cursor.last_matched, Some(1));
        assert_eq!(cursor.matched, 1);
        cursor = mark;
        assert_eq!(cursor, TokenCursor::default());
        assert_eq!(work, Some(2));
        Ok(())
    }

    #[test]
    fn traversal_exact_and_one_short_denial_keep_source_record() -> Result<(), String> {
        let tokens = tokens();
        let mut exact = TokenCursor::default();
        let mut enough = Some(2);
        exact.skip_hidden(&tokens, |token| Ok(token.text == " "), &mut enough)?;
        assert_eq!(exact.position, 1);
        assert_eq!(enough, Some(0));
        let mut denied = TokenCursor::default();
        let error = denied
            .skip_hidden(&tokens, |token| Ok(token.text == " "), &mut Some(1))
            .err()
            .ok_or("Missing expected admission denial")?;
        assert!(error.contains("budget"));
        assert_eq!(denied.position, 1);
        assert_eq!(denied.last_matched, None);
        assert_eq!(tokens[denied.position].text, "a");
        Ok(())
    }
}
