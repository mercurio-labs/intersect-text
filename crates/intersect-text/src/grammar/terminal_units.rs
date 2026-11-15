//! Ordinary string terminals borrow their proven ASCII prefix. Unicode tails
//! and arbitrary source-unit iterators use the canonical lazy UTF-16 buffer.
use std::str::EncodeUtf16;

pub(super) trait UnitInput {
    fn unit(&mut self, offset: usize) -> Option<u16>;
}

pub(super) struct BufferedUnits<I> {
    remaining: I,
    units: Vec<u16>,
}

impl<I> BufferedUnits<I> {
    pub(super) fn new(remaining: I) -> Self {
        Self {
            remaining,
            units: Vec::new(),
        }
    }
}

impl<I: Iterator<Item = u16>> UnitInput for BufferedUnits<I> {
    fn unit(&mut self, offset: usize) -> Option<u16> {
        while self.units.len() <= offset {
            self.units.push(self.remaining.next()?);
        }
        self.units.get(offset).copied()
    }
}

pub(super) struct StringUnits<'a> {
    input: &'a str,
    ascii: usize,
    tail: Option<BufferedUnits<EncodeUtf16<'a>>>,
    /// One past the largest unit offset requested so far.
    requested: usize,
}

impl<'a> StringUnits<'a> {
    pub(super) fn new(input: &'a str) -> Self {
        Self {
            input,
            ascii: 0,
            tail: None,
            requested: 0,
        }
    }

    /// The leading bytes that decided every answer so far, when they are
    /// all ASCII and no request reached the end of the input: byte and unit
    /// offsets then coincide and the input's suffix was never observed.
    pub(super) fn observed_ascii_prefix(&self) -> Option<usize> {
        (self.tail.is_none() && self.requested <= self.ascii).then_some(self.requested)
    }

    pub(super) fn byte_endpoint(&self, end: usize) -> Result<usize, String> {
        if end > 0 && end <= self.ascii {
            Ok(end)
        } else {
            // Keep zero-length and partial-surrogate endpoint refusal exactly
            // as in the original string matcher.
            super::utf8_endpoint(self.input, end)
        }
    }
}

impl UnitInput for StringUnits<'_> {
    fn unit(&mut self, offset: usize) -> Option<u16> {
        let bytes = self.input.as_bytes();
        self.requested = self.requested.max(offset.saturating_add(1));
        if self.tail.is_none() && offset >= self.ascii {
            // Validate only through the requested unit, never the unconsumed
            // source suffix. An arbitrary forward request must prove every
            // intervening byte before byte and UTF-16 offsets can coincide.
            let limit = offset.saturating_add(1).min(bytes.len());
            while self.ascii < limit {
                if !bytes[self.ascii].is_ascii() {
                    // All preceding bytes are ASCII, so this is a UTF-8
                    // boundary and also the exact UTF-16 tail offset.
                    self.tail = Some(BufferedUnits::new(self.input[self.ascii..].encode_utf16()));
                    break;
                }
                self.ascii += 1;
            }
        }
        if offset < self.ascii {
            Some(u16::from(bytes[offset]))
        } else {
            self.tail.as_mut()?.unit(offset - self.ascii)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn string_units_match_utf16_at_forward_repeated_and_rewound_offsets() {
        for input in [
            "",
            "ascii",
            "éabc",
            "aéz",
            "ab😀z",
            "😀😁",
            "\0\r\n",
            "a\u{10ffff}z",
        ] {
            let expected: Vec<_> = input.encode_utf16().collect();
            let offsets: Vec<_> = (0..=expected.len() + 1).collect();
            for visits in [
                offsets.clone(),
                offsets.iter().copied().rev().collect(),
                vec![usize::MAX, 0, 1, 0, 3, 2, 1],
            ] {
                let mut reader = StringUnits::new(input);
                for offset in visits {
                    assert_eq!(
                        reader.unit(offset),
                        expected.get(offset).copied(),
                        "{input:?} at {offset}"
                    );
                    for end in 0..=expected.len() + 1 {
                        assert_eq!(
                            reader.byte_endpoint(end),
                            super::super::utf8_endpoint(input, end)
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn ascii_prefix_is_borrowed_only_through_requested_units() {
        let mut reader = StringUnits::new("abc😀tail");
        assert_eq!(reader.unit(1), Some(98));
        assert_eq!(reader.ascii, 2);
        assert!(reader.tail.is_none());
        assert_eq!(reader.byte_endpoint(2), Ok(2));
        assert_eq!(reader.unit(3), Some(0xd83d));
        assert_eq!(reader.ascii, 3);
        assert_eq!(reader.tail.as_ref().unwrap().units, vec![0xd83d]);
        assert!(reader.byte_endpoint(4).is_err());
        assert_eq!(reader.unit(4), Some(0xde00));
        assert_eq!(reader.byte_endpoint(5), Ok(7));
        assert_eq!(reader.unit(0), Some(97));
        assert_eq!(reader.tail.as_ref().unwrap().units.len(), 2);
        assert!(reader.byte_endpoint(0).is_err());
    }

    #[test]
    fn arbitrary_iterator_keeps_original_reads_and_non_fused_eof_behavior() {
        let reads = Cell::new(0);
        let mut values = [Some(97), None, Some(0xd800), None].into_iter();
        let mut reader = BufferedUnits::new(std::iter::from_fn(|| {
            reads.set(reads.get() + 1);
            values.next().flatten()
        }));
        assert_eq!(reader.unit(0), Some(97));
        assert_eq!(reader.unit(0), Some(97));
        assert_eq!(reads.get(), 1);
        assert_eq!(reader.unit(1), None);
        assert_eq!(reads.get(), 2);
        assert_eq!(reader.unit(1), Some(0xd800));
        assert_eq!(reads.get(), 3);
        assert_eq!(reader.unit(0), Some(97));
        assert_eq!(reader.unit(2), None);
        assert_eq!(reads.get(), 4);
    }
}
