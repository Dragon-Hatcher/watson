use std::{fmt::Debug, ops::Range};

use ustr::Ustr;

/// Identifies a source file by its path from the root, given in the form
/// path.from.root
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceId(Ustr);

impl SourceId {
    pub fn new(path: Ustr) -> Self {
        Self(path)
    }

    pub fn path(&self) -> Ustr {
        self.0
    }

    pub fn start_loc(&self) -> Location {
        Location::new(*self, SourceOffset::new(0))
    }
}

/// A location in an unknown source. Identified by its byte offset in that source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceOffset(usize);

impl SourceOffset {
    pub fn new(byte_offset: usize) -> Self {
        Self(byte_offset)
    }

    pub fn byte_offset(&self) -> usize {
        self.0
    }

    pub fn forward(&self, bytes: usize) -> Self {
        Self(self.0 + bytes)
    }
}

/// A location within a specific source. Identified by the source's `SourceKey`
/// and the location's `SourceOffset`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Location {
    source: SourceId,
    offset: SourceOffset,
}

impl Location {
    pub fn new(source: SourceId, offset: SourceOffset) -> Self {
        Self { source, offset }
    }

    pub fn source(&self) -> SourceId {
        self.source
    }

    pub fn _offset(&self) -> SourceOffset {
        self.offset
    }

    pub fn byte_offset(&self) -> usize {
        self.offset.byte_offset()
    }

    pub fn forward(&self, bytes: usize) -> Self {
        Self::new(self.source, self.offset.forward(bytes))
    }

    pub fn max(&self, other: &Self) -> Self {
        if self.byte_offset() > other.byte_offset() {
            *self
        } else {
            *other
        }
    }
}

impl Debug for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Location({}:{})",
            self.source.0,
            self.offset.byte_offset()
        )
    }
}

/// A range within a specific source. Identified by the start byte offset
/// (inclusive) and the end byte offset (exclusive).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    start: Location,
    end: Location,
}

impl Span {
    pub fn new(start: Location, end: Location) -> Self {
        assert_eq!(start.source(), end.source());

        Self { start, end }
    }

    pub fn source(&self) -> SourceId {
        self.start.source()
    }

    pub fn start(&self) -> Location {
        self.start
    }

    pub fn end(&self) -> Location {
        self.end
    }

    pub fn bytes(&self) -> Range<usize> {
        self.start().byte_offset()..self.end().byte_offset()
    }

    pub fn union(&self, end: Span) -> Span {
        assert_eq!(self.source(), end.source());
        Span::new(self.start(), end.end())
    }
}

impl Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Span({}:{}-{})",
            self.source().0,
            self.start.byte_offset(),
            self.end.byte_offset()
        )
    }
}
