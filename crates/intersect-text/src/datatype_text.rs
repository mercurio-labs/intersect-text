//! Handwritten Xtext datatype-token accumulation, shared by text consumers.
//! Contract: fixtures/xtext/datatype-contract (pinned independent Xtext oracle).
//! Raw spelling and scalar checkpoints are consumed by the shared text machine.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TextCheckpoint {
    length: usize,
    start: Option<usize>,
    expected: Option<usize>,
}

#[derive(Debug, Default)]
pub(crate) struct DatatypeText {
    text: String,
    start: Option<usize>,
    expected: Option<usize>,
}

impl DatatypeText {
    pub(crate) fn checkpoint(&self) -> TextCheckpoint {
        TextCheckpoint {
            length: self.text.len(),
            start: self.start,
            expected: self.expected,
        }
    }

    pub(crate) fn rollback(&mut self, checkpoint: TextCheckpoint) -> Result<(), String> {
        if checkpoint.length > self.text.len() || !self.text.is_char_boundary(checkpoint.length) {
            return Err("Invalid datatype text checkpoint".into());
        }
        self.text.truncate(checkpoint.length);
        self.start = checkpoint.start;
        self.expected = checkpoint.expected;
        Ok(())
    }

    /// Offsets count UTF-16 code units, while checkpoints count Rust string bytes.
    /// Keep raw lexical spelling; terminal value conversion is a separate layer.
    pub(crate) fn push(&mut self, text: &str, offset: usize) -> Result<(), String> {
        let expected = offset
            .checked_add(text.encode_utf16().count())
            .ok_or_else(|| "Datatype token offset overflow".to_string())?;
        if self.expected.is_some_and(|previous| offset > previous) {
            self.text.push(' ');
        }
        self.text.push_str(text);
        self.start.get_or_insert(offset);
        self.expected = Some(expected);
        Ok(())
    }

    /// Preserve nested rule boundaries: upstream merges the child text and start,
    /// not its last source-token end. Flattening produces different gap spacing.
    pub(crate) fn merge(&mut self, child: &Self) -> Result<(), String> {
        if child.text.is_empty() {
            return Ok(());
        }
        let start = child
            .start
            .ok_or_else(|| "Datatype text has no source offset".to_string())?;
        self.push(&child.text, start)
    }

    /// Exact first matched token offset, before whitespace-gap conversion.
    pub(crate) fn source_start(&self) -> Option<usize> {
        self.start
    }

    pub(crate) fn into_text(self) -> Option<String> {
        if self.text.is_empty() {
            None
        } else {
            Some(self.text)
        }
    }
}

/// Build source positions once; each source segment is visited at most once.
/// Native model diagnostics keep byte offsets, but Xtext token merging uses UTF-16.
pub(crate) fn utf16_offsets(
    source: &str,
    starts: impl IntoIterator<Item = usize>,
) -> Result<Vec<usize>, String> {
    let mut previous = 0;
    let mut offset = 0;
    let mut result = Vec::new();
    for start in starts {
        let segment = source
            .get(previous..start)
            .ok_or_else(|| "Invalid datatype token source offset".to_string())?;
        offset += segment.encode_utf16().count();
        result.push(offset);
        previous = start;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(parts: &[(&str, usize)]) -> DatatypeText {
        let mut value = DatatypeText::default();
        for (text, offset) in parts {
            value.push(text, *offset).unwrap();
        }
        value
    }

    #[test]
    fn pinned_oracle_compact_hidden_raw_quotes_and_utf16() {
        // These exact values and offsets are independently observed in the
        // committed Java oracle; they are not inferred from this implementation.
        for (parts, expected, start, end) in [
            (vec![("P", 0), ("::", 1), ("A", 3)], "P::A", 0, 4),
            (vec![("P", 2), ("::", 6), ("A", 11)], "P :: A", 2, 12),
            (vec![("\"B\\tC\"", 0)], "\"B\\tC\"", 0, 6),
            (vec![("\"😀\"", 0), ("::", 4), ("A", 6)], "\"😀\"::A", 0, 7),
        ] {
            let result = tokens(&parts);
            assert_eq!(result.start, Some(start));
            assert_eq!(result.expected, Some(end));
            assert_eq!(result.into_text().as_deref(), Some(expected));
        }
    }

    #[test]
    fn pinned_oracle_nested_compressed_gap_differs_from_flattening() {
        // Source spelling A/*gap*/::B. Child ends at B, outer then consumes dot.
        let child = tokens(&[("A", 0), ("::", 8), ("B", 10)]);
        let mut nested = DatatypeText::default();
        nested.merge(&child).unwrap();
        nested.push(".", 11).unwrap();
        assert_eq!(nested.expected, Some(12));
        assert_eq!(nested.into_text().as_deref(), Some("A ::B ."));
        let flat = tokens(&[("A", 0), ("::", 8), ("B", 10), (".", 11)]);
        assert_eq!(flat.into_text().as_deref(), Some("A ::B."));
        let mut empty = DatatypeText::default();
        empty.merge(&DatatypeText::default()).unwrap();
        assert_eq!(empty.checkpoint(), TextCheckpoint::default());
        assert_eq!(empty.into_text(), None);
    }

    #[test]
    fn speculative_nested_merge_restores_text_and_gap_offsets() {
        let mut outer = tokens(&[("\"😀\"", 0)]);
        let checkpoint = outer.checkpoint();
        outer
            .merge(&tokens(&[("A", 10), ("::", 18), ("B", 20)]))
            .unwrap();
        outer.push("!", 21).unwrap();
        outer.rollback(checkpoint).unwrap();
        assert_eq!(outer.checkpoint(), checkpoint);
        outer.push("::", 4).unwrap();
        outer.push("C", 6).unwrap();
        assert_eq!(outer.into_text().as_deref(), Some("\"😀\"::C"));
    }

    #[test]
    fn source_offset_map_is_linear_unicode_aware_and_validates_boundaries() {
        let source = "é /*😀*/ \"😀\"::A";
        let starts = [
            0,
            source.find('"').unwrap(),
            source.find("::").unwrap(),
            source.len() - 1,
        ];
        let offsets = utf16_offsets(source, starts).unwrap();
        assert_eq!(offsets, vec![0, 9, 13, 15]);
        let mut text = DatatypeText::default();
        text.push("\"😀\"", offsets[1]).unwrap();
        text.push("::", offsets[2]).unwrap();
        text.push("A", offsets[3]).unwrap();
        assert_eq!(text.into_text().as_deref(), Some("\"😀\"::A"));
        assert!(utf16_offsets(source, [1]).is_err());
        assert!(utf16_offsets(source, [source.len() + 1]).is_err());
        assert!(utf16_offsets(source, [2, 0]).is_err());
        assert_eq!(utf16_offsets(source, [0, 0]).unwrap(), vec![0, 0]);
    }

    #[test]
    fn invalid_offsets_and_checkpoints_do_not_mutate_accumulated_text() {
        let mut text = tokens(&[("é", 0)]);
        let before = text.checkpoint();
        assert!(text.push("A", usize::MAX).unwrap_err().contains("overflow"));
        assert_eq!(text.checkpoint(), before);
        let mut invalid = before;
        invalid.length = 1;
        assert!(text.rollback(invalid).is_err());
        assert_eq!(text.checkpoint(), before);
        invalid.length = 3;
        assert!(text.rollback(invalid).is_err());
        assert_eq!(text.into_text().as_deref(), Some("é"));
    }
}
