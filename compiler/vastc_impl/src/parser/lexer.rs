use crate::parser::token::{Errno, NumericTypeQualifier, Token, TokenFlags, TokenType, KEYWORD_MAP, TREE_ALL_SYMBOLS, NumericLiteralMetadata};
use crate::spec::resolve_escaped_char;
use std::ops::Deref;
use vastc_supplemental::source::{StringCursor, OptionalChar, Source, SourceSpan, SourceSpanOps};

/// The result type of the `next_token` method. This intentionally does not
/// return a successful token on success, as Ok may also represent no errors,
/// but no token parsed. The authoritative state on tokens consumed is `self.tokens`
pub type NextTkResult = Result<(), /* index into `tokens` */ usize>;

/// The generic result which does not specify any result error,
/// only the range where it occurred.
pub type GenericConsumeResult<S> = Result<S, Option<SourceSpan>>;

/// Represents the lexer configuration and state.
///
/// `unit` is the lifetime of the compilation unit and thus the source.
pub struct Lexer<'unit> {
  /// The loaded source code to be compiled.
  source: &'unit Source,
  /// The current (owned) cursor into the source string.
  cursor: StringCursor<'unit>,
  /// The result vector of tokens
  tokens: Vec<Token>,
  /// The vector of only error tokens as indices into `self.tokens`
  error_tokens: Vec<usize>
}

impl<'unit> Lexer<'unit> {
  pub fn new(source: &'unit Source) -> Self {
    Lexer {
      source,
      cursor: StringCursor::new(source),
      tokens: Vec::new(),
      error_tokens: Vec::new()
    }
  }

  pub fn tokens(&self) -> &Vec<Token> {
    &self.tokens
  }
  
  pub fn take_tokens(self) -> Vec<Token> {
    self.tokens
  }

  pub fn error_count(&self) -> usize {
    self.error_tokens.len()
  }

  pub fn errors(&self) -> impl Iterator<Item = &Token> + '_ {
    self.error_tokens.iter().map(|&idx| &self.tokens[idx])
  }

  pub fn cursor(&self) -> &StringCursor<'_> {
    &self.cursor
  }

  /// Parse the next token in the source, ahead from the
  /// current cursor.
  pub fn next_token(&mut self) -> NextTkResult {
    // consume whitespace
    self.consume_whitespace();
    if self.cursor.done() {
      return Ok(())
    }

    let idx0 = self.cursor.index();

    // check for comments
    if let Some(_) = self.cursor.consume_block_comment() {
      return Ok(())
    }

    if let Some(_) = self.cursor.consume_line_comment() {
      return Ok(())
    }

    // try parse special chars
    if let Some((tk, span)) = TREE_ALL_SYMBOLS.deref().read_or_restore(&mut self.cursor) {
      return Ok(_ = self.make(tk, Some(span)));
    }

    // try parse numeric literal
    if self.cursor.peek().is_digit(10) {
      // valid formats:
      // ints   -- [0(x/b/o)]digits[u][8/16/32/64/...]
      // floats -- digits[.digits][f][8/16/32/64/...]

      // check for 0[x/b/...]
      let mut radix = 10;
      if self.cursor.peek() == '0' && self.cursor.peek_next().is_alphabetic() {
        radix = match self.cursor.peek_next() {
          'x' => 16,
          'b' => 2,
          'o' => 8,
          _   => { self.err(Errno::IllegalRadixChar, "unknown radix specifier", Some(self.cursor.here())); 10 },
        };

        self.cursor.advance_by(2);
      }

      // bookkeeping about literal info
      let mut is_float = false;
      let mut type_qual = NumericTypeQualifier::Infer;

      // start parsing digits!
      let content_start_index = self.cursor.index();
      if self.cursor.peek() == '-' {
        self.cursor.advance(); // negative sign parsed later
      }

      while self.cursor.peek().is_digit(radix) {
        self.cursor.advance();
      }

      // detect dot character, and check that it isnt
      // part of a range or member access
      if self.cursor.peek() == '.' {
        if /* member access */ self.cursor.peek_next().is_alphabetic() ||
           /* range spec */ self.cursor.peek_next() == '.' {
          return Ok(_ = self.make(TokenType::NumericLiteral(content_start_index..self.cursor.index(), NumericLiteralMetadata {
            is_float,
            type_qual,
            radix: radix as u16
          }), Some(idx0..self.cursor.index())))
        }

        // classified as decimal point
        is_float = true;
        self.cursor.advance();

        // collect decimal range
        while self.cursor.peek().is_digit(radix) {
          self.cursor.advance();
        }
      }

      let end_index = self.cursor.index();

      // parse type qual and width
      if self.cursor.peek().is_alphabetic() {
        type_qual = match self.cursor.peek() {
          'f' => { self.cursor.advance(); NumericTypeQualifier::Float(self.cursor.parse_u32().unwrap_or(32)) }
          'i' => { self.cursor.advance(); NumericTypeQualifier::Int(self.cursor.parse_u32().unwrap_or(32)) }
          'u' => { self.cursor.advance(); NumericTypeQualifier::Unsigned(self.cursor.parse_u32().unwrap_or(32)) }
          _   => { self.cursor.advance(); NumericTypeQualifier::Infer }
        };

        if type_qual == NumericTypeQualifier::Infer {
          self.err(Errno::InvalidTypeQualifier, "invalid numeric literal type qualifier", Some(idx0..self.cursor.index()));
        }
      }

      // classified as decimal point, if radix is not 10,
      // throw an error for invalid radix
      if is_float && radix != 10 {
        self.err(Errno::IllegalDecimalRadix, "float literals must be in decimal radix", Some(idx0..self.cursor.index()));
      }

      if let NumericTypeQualifier::Float(_) = type_qual {
        // we set it after the check above because we want hexadecimal int literals to be convertible to floats
        is_float = true;
      }

      return Ok(_ = self.make(TokenType::NumericLiteral(content_start_index..end_index, NumericLiteralMetadata {
        is_float,
        type_qual,
        radix: radix as u16
      }), Some(idx0..self.cursor.index())))
    }

    // try parse identifier
    if let Some(span) = self.cursor.consume_identifier() {
      let str: &str = self.source.slice(span.clone());

      // check for keyword
      if let Some(kw) = KEYWORD_MAP.get(str) {
        return Ok(_ = self.make(kw.clone(), Some(span)));
      }

      // return as identifier token
      return Ok(_ = self.make(TokenType::Identifier, Some(span)));
    }

    // try parse string literal
    if self.cursor.peek() == '"' {
      let index = self.cursor.index();
      self.cursor.advance();
      let mut buf = String::new();

      loop {
        let sub_idx = self.cursor.index(); // index of this 'unit' of the literal
        let ch = self.cursor.peek();
        if ch.is_eof() {
          return Ok(_ = self.err(Errno::UnterminatedLiteral, "unterminated string literal before eof",
                           Some(index..self.cursor.index())));
        }

        // check for end of string
        if ch == '"' {
          self.cursor.advance();
          return Ok(_ = self.make(TokenType::StringLiteral(buf), Some(index..self.cursor.index())));
        }

        // check for escaped
        if ch == '\\' {
          self.cursor.advance();

          // use spec-defined shared char sequence resolver,
          // which leaves the cursor after the literal
          if let Some(ch) = resolve_escaped_char(&mut self.cursor) {
            buf.push(ch);
          } else {
            _ = self.err(Errno::IllegalEscapedChar, "illegal escape by character in str literal", Some(sub_idx..self.cursor.index()))
          }

          continue
        }

        // just add character to the buffer and advance
        buf.push(ch);
        self.cursor.advance();
      }
    }

    // try parse char literal
    if self.cursor.peek() == '\'' {
      self.cursor.advance().expect().map_err(|_| self.err(Errno::UnterminatedLiteral, "unterminated char literal before eof", Some(idx0..self.cursor.index())))?;

      let idx = self.cursor.index();

      // nothing special, just add char token
      if self.cursor.peek() != '\\' {
        let ch = self.cursor.peek();
        self.cursor.advance();
        self.expect_char_and_always_advance('\'').map_err(|s| self.err(Errno::IllegalCharacter, "unexpected character, expected `'` to close char literal", s))?;
        return Ok(_ = self.make(TokenType::CharLiteral(ch), Some(idx0..self.cursor.index())))
      }

      // use spec-defined shared char sequence resolver,
      // which leaves the cursor after the literal
      if let Some(ch) = resolve_escaped_char(&mut self.cursor) {
        self.make(TokenType::CharLiteral(ch), Some(idx0..self.cursor.index() + 1));
      } else {
        _ = self.err(Errno::IllegalEscapedChar, "illegal escape by character in char literal", Some(idx..self.cursor.index()))
      }

      // expect closing '
      self.expect_char_and_always_advance('\'').map_err(|s| self.err(Errno::IllegalCharacter, "expected `'` to close char literal", s))?;
      return Ok(())
    }

    // invalid character: expand or make error token
    if let Some(last_tk) = self.tokens.last_mut() {
      let cursor = &mut self.cursor;
      if let TokenType::Error(errno, _) = last_tk.ty &&
        errno == Errno::IllegalCharacter
      {
        // expand last token to cover this range (avoid polluting error counts)
        last_tk.location = match &last_tk.location {
          None => Some(cursor.here().into()),
          Some(loc) => Some(loc.join(cursor.here()))
        };

        cursor.advance();
        return Ok(()) // todo: hack, cba
      }
    }

    let loc = self.cursor.here_and_advance();
    Err(self.err(Errno::IllegalCharacter, "unexpected character(s) in tokenization", Some(loc)))
  }

  /// Parse the whole source string into a stream of tokens.
  pub fn tokenize(&mut self) -> &mut Self {
    while !self.cursor.done() {
      _ = self.next_token();
    }

    self
  }

  fn err(&mut self, errno: Errno, msg: &'static str, loc: Option<SourceSpan>) -> usize {
    self.make(TokenType::Error(errno, msg), loc)
  }

  fn make(&mut self, ty: TokenType, loc: Option<SourceSpan>) -> usize {
    let idx = self.tokens.len();
    let slot: &mut Token = self.tokens.push_mut(Token {
      ty,
      flags: 0,
      location: loc,
    });

    if let TokenType::Error(_, _) = slot.ty {
      self.error_tokens.push(idx);
    }

    idx
  }

  fn consume_whitespace(&mut self) -> &mut Self {
    let mut apply: u32 = 0; // the flags to be applied to the last token
    while !self.cursor.done() && self.cursor.peek().is_whitespace() {
      let char = self.cursor.peek();
      if char == '\n' { apply |= TokenFlags::TAIL_NEWLINE; }
      apply |= TokenFlags::TAIL_SPACE;
      self.cursor.advance();
    }

    if apply != 0 && let Some(tk) = self.tokens.last_mut() {
      tk.flags |= apply;
    }

    self
  }

  fn expect_mut(&mut self, predicate: impl FnOnce(&mut Self) -> bool) -> GenericConsumeResult<()> {
    let idx0 = self.cursor.index();
    if !predicate(self) {
      Err(Some(idx0..self.cursor.index() + 1))
    } else {
      Ok(())
    }
  }

  fn expect_char_and_always_advance(&mut self, ch: char) -> GenericConsumeResult<()> {
    let res = if self.cursor.peek() != ch {
      Err(Some(self.cursor.here()))
    } else {
      Ok(())
    };

    self.cursor.advance();
    res
  }
}