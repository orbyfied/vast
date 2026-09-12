use std::fmt::{Debug, Formatter};
use std::ops::Range;
use crate::parse::CharIterator;

pub trait SourceSpanOps {
  fn start(&self) -> SourceIndex;
  fn end(&self) -> SourceIndex;
  fn join(&self, other: SourceSpan) -> SourceSpan;
  fn contains_span(&self, other: &SourceSpan) -> bool;
}

impl SourceSpanOps for SourceSpan {
  fn start(&self) -> SourceIndex {
    self.start
  }

  fn end(&self) -> SourceIndex {
    self.end
  }

  /// Returns a span covering both spans.
  fn join(&self, other: SourceSpan) -> SourceSpan {
    Self {
      start: if self.start < other.start {
        self.start
      } else {
        other.start
      },

      end: if self.end > other.end {
        self.end
      } else {
        other.end
      },
    }
  }

  /// Returns true if this span completely contains `other`.
  fn contains_span(&self, other: &SourceSpan) -> bool {
    self.start <= other.start && self.end >= other.end
  }
}

/// Immutable source text together with a character index to byte offset table.
///
/// `boundaries[i]` is the byte offset at which character `i` begins.
///
/// The boundary table is sized `length + 1`, where the added entry is an EOF marker.
#[derive(Clone)]
pub struct Source {
  text: String,
  identifier: String,
  boundaries: Vec<SourceIndex>,
}

impl Debug for Source {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str("Source(id: ")?;
    f.write_str(&*self.identifier)?;
    f.write_str(", len: ")?;
    f.write_fmt(format_args!("{}", self.len()))?;
    f.write_str("}")
  }
}

/// End-of-file sentinel value, provides just slightly more ergonomic parsing
pub const EOF: char = '\0';

/// The canonical type for char indices into a source string
pub type SourceIndex = u32;

/// The canonical type for spans of char indices
pub type SourceSpan = Range<SourceIndex>;

impl Source {
  pub fn new(id: impl Into<String>, text: impl Into<String>) -> Self {
    let text = text.into();

    // One entry for every Unicode scalar value, plus the final
    // byte offset representing EOF.
    let mut boundaries = Vec::with_capacity(text.len() + 1);

    for (byte_index, _) in text.char_indices() {
      boundaries.push(byte_index as SourceIndex);
    }

    boundaries.push(text.len() as SourceIndex);

    Self { text, identifier: id.into(), boundaries }
  }

  /// The complete source text.
  pub fn text(&self) -> &str {
    &self.text
  }

  /// Number of Unicode scalar values in the source.
  pub fn len(&self) -> SourceIndex {
    (self.boundaries.len() - 1) as SourceIndex
  }

  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  /// Returns the byte offset corresponding to a character index.
  ///
  /// The EOF character index is valid.
  pub fn byte_index(&self, char_index: SourceIndex) -> usize {
    self.boundaries[char_index as usize] as usize
  }

  /// Returns the character at `index`.
  pub fn char_at(&self, index: SourceIndex) -> char {
    if index >= self.len() {
      return EOF;
    }

    let start = self.byte_index(index);
    let end = self.byte_index(index + 1);

    self.text[start..end].chars().next().unwrap_or(EOF)
  }

  pub fn char_at_0(&self, index: SourceIndex) -> Option<char> {
    if index >= self.len() {
      return None;
    }

    let start = self.byte_index(index);
    let end = self.byte_index(index + 1);

    self.text[start..end].chars().next()
  }

  /// Converts a character-indexed span into a byte range.
  pub fn byte_range(&self, span: SourceSpan) -> Range<usize> {
    assert!(span.end <= self.len());

    self.byte_index(span.start)..self.byte_index(span.end)
  }

  /// Gets the text corresponding to a character-indexed span.
  pub fn slice(&self, span: SourceSpan) -> &str {
    let range = self.byte_range(span);
    &self.text[range]
  }

  /// Creates a cursor at the beginning of the source.
  pub fn cursor(&self) -> Cursor<'_> {
    Cursor::new(self)
  }

  /// Creates a cursor at a particular character index.
  pub fn cursor_at(&self, index: SourceIndex) -> Cursor<'_> {
    Cursor::at(self, index)
  }

  /// Find the semantic location for the given character index,
  /// this is not a cheap operation.
  pub fn position_of(&self, index: SourceIndex) -> Position {
    let byte_position = self.byte_index(index);

    // analyze string to find line and column todo
    let line = 0;
    let column = index - line;

    Position { index, byte_position, line, column }
  }

  /// Convert the span to a segment.
  pub fn as_segment(&self, span: SourceSpan) -> Segment {
    let start = self.position_of(span.start);
    let end = self.position_of(span.end);
    Segment { start, end }
  }
}

/// A cursor over a `Source`.
#[derive(Debug, Clone, Copy)]
pub struct Cursor<'a> {
  source: &'a Source,
  index: SourceIndex,
}

impl<'a> Cursor<'a> {
  pub fn new(source: &'a Source) -> Self {
    Self { source, index: 0 }
  }

  pub fn at(source: &'a Source, index: SourceIndex) -> Self {
    assert!(index <= source.len());

    Self { source, index }
  }

  // -------------------------------------------------------------------------
  // Position
  // -------------------------------------------------------------------------

  pub fn source(&self) -> &'a Source {
    self.source
  }

  pub fn index(&self) -> SourceIndex {
    self.index
  }

  pub fn byte_index(&self) -> usize {
    self.source.byte_index(self.index)
  }

  pub fn is_eof(&self) -> bool {
    self.index >= self.source.len()
  }

  pub fn done(&self) -> bool {
    self.is_eof()
  }

  pub fn remaining(&self) -> SourceSpan {
    self.index..self.source.len()
  }

  pub fn mark(&self) -> SourceIndex {
    self.index
  }

  pub fn span_from(&self, start: SourceIndex) -> SourceSpan {
    start..self.index
  }

  // -------------------------------------------------------------------------
  // Lookahead
  // -------------------------------------------------------------------------

  pub fn peek(&self) -> char {
    self.source.char_at(self.index)
  }

  pub fn peek_next(&self) -> char {
    self.source.char_at(self.index + 1)
  }

  pub fn peek_n(&self, n: SourceIndex) -> char {
    self.source.char_at(self.index + n)
  }

  pub fn peek_0(&self) -> Option<char> {
    self.source.char_at_0(self.index)
  }

  pub fn peek_next_0(&self) -> Option<char> {
    self.source.char_at_0(self.index + 1)
  }

  pub fn peek_n_0(&self, n: SourceIndex) -> Option<char> {
    self.source.char_at_0(self.index + n)
  }

  pub fn starts_with(&self, text: &str) -> bool {
    self.source.slice(self.remaining()).starts_with(text)
  }

  // -------------------------------------------------------------------------
  // Movement
  // -------------------------------------------------------------------------

  pub fn advance(&mut self) -> char {
    if !self.done() {
      self.index += 1
    }

    self.peek()
  }

  pub fn advance_0(&mut self) -> Option<char> {
    if !self.done() {
      self.index += 1
    }

    self.peek_0()
  }

  pub fn advance_by(&mut self, count: SourceIndex) -> char {
    assert!(self.index + count <= self.source.len());
    self.index += count;
    self.peek()
  }

  pub fn retreat(&mut self) -> char {
    assert!(self.index > 0);
    self.index -= 1;
    self.peek()
  }

  pub fn set_index(&mut self, index: SourceIndex) {
    assert!(index <= self.source.len());
    self.index = index;
  }

  // -------------------------------------------------------------------------
  // Consuming
  // -------------------------------------------------------------------------

  pub fn consume_if(&mut self, expected: char) -> bool {
    if self.peek() == expected {
      self.index += 1;
      true
    } else {
      false
    }
  }

  pub fn consume_str(&mut self, expected: &str) -> bool {
    if !self.starts_with(expected) {
      return false;
    }

    let char_count = expected.chars().count() as SourceIndex;
    self.index += char_count;

    true
  }

  pub fn consume_while<F>(&mut self, mut predicate: F) -> SourceSpan
  where
    F: FnMut(char) -> bool,
  {
    let start = self.index;

    while let Some(ch) = self.peek_0() {
      if !predicate(ch) {
        break;
      }

      self.index += 1;
    }

    self.span_from(start)
  }

  pub fn consume_while_1<F>(&mut self, predicate: F) -> Option<SourceSpan>
  where
    F: FnMut(char) -> bool,
  {
    let span = self.consume_while(predicate);

    if span.is_empty() { None } else { Some(span) }
  }

  pub fn here(&self) -> SourceSpan {
    self.index..self.index + 1
  }

  // -------------------------------------------------------------------------
  // Whitespace
  // -------------------------------------------------------------------------

  pub fn consume_whitespace(&mut self) -> SourceSpan {
    self.consume_while(char::is_whitespace)
  }

  pub fn consume_whitespace_1(&mut self) -> Option<SourceSpan> {
    self.consume_while_1(char::is_whitespace)
  }

  // -------------------------------------------------------------------------
  // Identifiers
  // -------------------------------------------------------------------------

  pub fn consume_identifier(&mut self) -> Option<SourceSpan> {
    let start = self.index;

    let first = self.peek_0()?;

    if !is_identifier_start(first) {
      return None;
    }

    self.index += 1;

    while let Some(ch) = self.peek_0() {
      if !is_identifier_continue(ch) {
        break;
      }

      self.index += 1;
    }

    Some(self.span_from(start))
  }

  // -------------------------------------------------------------------------
  // Comments
  // -------------------------------------------------------------------------

  pub fn consume_line_comment(&mut self) -> Option<SourceSpan> {
    let start = self.index;

    if !self.consume_str("//") {
      return None;
    }

    self.consume_while(|ch| ch != '\n');

    Some(self.span_from(start))
  }

  pub fn consume_block_comment(&mut self) -> Option<SourceSpan> {
    let start = self.index;

    if !self.consume_str("/*") {
      return None;
    }

    while !self.is_eof() {
      if self.consume_str("*/") {
        return Some(self.span_from(start));
      }

      self.advance();
    }

    None
  }

  // -------------------------------------------------------------------------
  // Expectations
  // -------------------------------------------------------------------------

  pub fn expect(&mut self, expected: char) -> Result<SourceSpan, CursorParsingError> {
    let start = self.index;

    if self.consume_if(expected) {
      Ok(self.span_from(start))
    } else {
      Err(CursorParsingError::UnexpectedCharacter {
        position: self.index,
        expected,
        found: self.peek(),
      })
    }
  }

  pub fn expect_str(&mut self, expected: &str) -> Result<SourceSpan, CursorParsingError> {
    let start = self.index;

    if self.consume_str(expected) {
      Ok(self.span_from(start))
    } else {
      Err(CursorParsingError::UnexpectedText {
        position: self.index,
        expected: expected.to_owned(),
      })
    }
  }

  // -------------------------------------------------------------------------
  // Speculative parsing
  // -------------------------------------------------------------------------

  pub fn try_consume<T, F>(&mut self, f: F) -> Option<T>
  where
    F: FnOnce(&mut Self) -> Option<T>,
  {
    let checkpoint = self.index;

    match f(self) {
      Some(value) => Some(value),

      None => {
        self.index = checkpoint;
        None
      }
    }
  }

  pub fn checkpoint(&self) -> Checkpoint {
    Checkpoint { index: self.index }
  }

  pub fn restore(&mut self, checkpoint: Checkpoint) {
    assert!(checkpoint.index <= self.source.len());
    self.index = checkpoint.index;
  }
}

pub trait OptionalChar {
  fn is_eof(&self) -> bool;
  fn expect(&self) -> Result<(), ()>;
  fn opt(&self) -> Option<char>;
}

impl OptionalChar for char {
  fn is_eof(&self) -> bool {
    *self == EOF
  }

  fn expect(&self) -> Result<(), ()> {
    if self.is_eof() {
      Err(())
    } else {
      Ok(())
    }
  }

  fn opt(&self) -> Option<char> {
    if self.is_eof() {
      None
    } else {
      Some(*self)
    }
  }
}

impl<'a> CharIterator for Cursor<'a> {
  fn index(&self) -> SourceIndex {
    self.index()
  }

  fn restore(&mut self, idx: SourceIndex) {
    self.set_index(idx)
  }

  fn advance_0(&mut self) -> Option<char> {
    self.advance_0()
  }

  fn peek_0(&self) -> Option<char> {
    self.peek_0()
  }

  fn done(&self) -> bool {
    self.is_eof()
  }
}

#[derive(Debug, Clone, Copy)]
pub struct Checkpoint {
  index: SourceIndex,
}

/// Errors which may be produced when parsing a construct using a Cursor.
#[derive(Debug)]
pub enum CursorParsingError {
  UnexpectedCharacter {
    position: SourceIndex,
    expected: char,
    found: char,
  },

  UnexpectedText {
    position: SourceIndex,
    expected: String,
  },
}

fn is_identifier_start(ch: char) -> bool {
  ch == '_' || ch.is_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
  ch == '_' || ch.is_alphanumeric()
}

/// Represents a semantic point position in source
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
  index: SourceIndex,           // The character index into the source file
  byte_position: usize, // The UTF-8 aligned byte position of the character

  line: SourceIndex,            // The line number of the character referred to by this position
  column: SourceIndex,          // The column number of the character referred to by this position
}

impl Position {
  pub fn index(&self) -> SourceIndex { self.index }
  pub fn byte_index(&self) -> usize { self.byte_position }
  pub fn line(&self) -> SourceIndex { self.line }
  pub fn column(&self) -> SourceIndex { self.column }
}

impl Into<SourceSpan> for Position {
  fn into(self) -> SourceSpan {
    self.index..self.index + 1
  }
}

/// Represents a semantic point position in source
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
  start: Position,
  end: Position, // exclusive
}

impl Segment {
  pub fn as_span(&self) -> SourceSpan {
    self.start.index..self.end.index
  }

  pub fn length(&self) -> SourceIndex {
    self.start.index - self.end.index
  }
}

impl Into<SourceSpan> for Segment {
  fn into(self) -> SourceSpan {
    self.start.index..self.end.index
  }
}

/*

  --------------------------------------------------------------------
   Tests
  --------------------------------------------------------------------

*/

mod tests {

}