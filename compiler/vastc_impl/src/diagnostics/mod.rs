use crate::parser::lexer::Lexer;
use crate::parser::token::{Token, TokenType};
use std::cmp::{max, min};
use std::io;
use std::ops::Range;
use vastc_supplemental::ansi;
use vastc_supplemental::ansi::{Attr, Color};
use vastc_supplemental::source::{Source, SourceIndex, SourceSpan, SourceSpanOps};
use crate::parser::token::TokenType::Colon;

pub struct UnitDiagnosticContext<'unit> {
  source: &'unit Source,
  lexer: Option<&'unit Lexer<'unit>>,
}

impl<'unit> UnitDiagnosticContext<'unit> {
  pub fn new(source: &'unit Source) -> Self {
    Self {
      source,
      lexer: None
    }
  }

  pub fn with_lexer(&mut self, lexer: &'unit Lexer) -> &mut Self {
    self.lexer = Some(lexer);
    self
  }
}

/// Represents the details of a generic diagnostic which may
/// have been produced by any stage in the compiler chain.
///
/// This should be the eventual owner of all diagnostic related memory.
#[derive(Clone, PartialEq, Debug)]
pub struct DiagnosticDetails {
  pub ty: DiagnosticType,
  /// The primary location of the diagnostic
  pub primary_location: SourceSpan,
  /// The error domain and numeral
  pub errno: GenericErrno,

  // todo: more flexible
  pub msg: &'static str,
}

#[derive(Clone, PartialEq, Copy, Debug)]
pub enum DiagnosticType {
  Error,
  Warn,
  Note,
}

#[derive(Clone, PartialEq, Copy, Debug)]
pub struct GenericErrno {
  pub domain: &'static str, // T -- Tokenization, P - Parser, etc...
  pub numeral: u32 // The actual error code
}

const DEFAULT_TRUE_COLOR: Color = Color::lit(0xDDDDDD);

pub fn print_diagnostic(writer: &mut impl io::Write, ctx: &UnitDiagnosticContext, det: &DiagnosticDetails) -> Result<(), io::Error> {
  let source = ctx.source;

  // prepare static resources
  let fg = det.ty.color();

  // formatting parameters
  const HORIZONTAL_WINDOW_EXPANSION: i64 = 25;

  let segment = source.as_segment(det.primary_location.clone());
  let exact_line_range = source.find_lines(segment.as_span());
  let line_range = exact_line_range.clone();

  // determine rectangle bounds
  let left_col = max(0, min(segment.start.column(), segment.end.column()) as i64 - HORIZONTAL_WINDOW_EXPANSION) as isize;
  let right_col = max(segment.start.column(), segment.end.column()) + HORIZONTAL_WINDOW_EXPANSION as SourceIndex;
  let rect_width = max((right_col - left_col as SourceIndex) + 25, 85);

  const H_BORDER_CHAR: char = '─';
  let rect_border_str: String = std::iter::repeat(H_BORDER_CHAR).take(rect_width).collect();

  // print top of segment
  writeln!(writer)?;

  // print position info and source window with highlight
  writeln!(writer, concat!(
    ansi!(bold), "{}▢",
    ansi!(_bold, gray),
    "       {}:{}"
  ), fg, source.identifier(), line_range.start + 1)?;

  writeln!(writer, concat!(
    "{}│",
    ansi!(gray),
    "      ╭─{}"
  ), fg, rect_border_str)?;

  let mut last_underline_columns: Option<Range<usize>> = None;
  let source_str = source.text();
  for line_index in line_range {
    let v_distance = exact_line_range.distance(line_index).abs();
    let line_number = line_index + 1;

    // print prefix before line
    write!(writer, concat!(
      "{}┆",
      ansi!(gray), "{: >5}",
      ansi!(reset, gray), " │ ",
      ansi!(reset)
    ), fg, line_number)?;

    let raw_line_span = source.lines()[line_index].clone();
    if raw_line_span.len() < 2 {
      continue
    }

    let line_span = raw_line_span.subrange(0, -1);
    let text_span = line_span.subrange(left_col, min(line_span.len() - 1, right_col) as isize);
    let exact_text_span = det.primary_location.clone();
    let exact_column_span = segment.start.column()..segment.end.column();
    last_underline_columns = Some(exact_column_span.clone());

    // print formatted slice using token data from lexer,
    // and process any special/formatted/annotated ranges
    // todo: annotated ranges, etc. for now, just red underline
    let mut current_token_index = 0;
    let mut last_formatting = (Color::Reset, Attr::Reset);
    for char_index in text_span {
      let col = char_index - line_span.start;
      let h_distance = exact_column_span.distance(col).abs();
      let byte_offset = source.byte_offset_of(char_index);
      let char = source_str[byte_offset..].chars().next().expect("no char in derived range?");

      // find the token associated with the current character index
      let current_token: Option<&Token> = if let Some(lexer) = ctx.lexer && current_token_index < lexer.tokens().len() {
        loop {
          if current_token_index >= lexer.tokens().len() {
            break None;
          }

          let tk = &lexer.tokens()[current_token_index];
          if let Some(span) = tk.location.clone() && char_index < span.end {
            if char_index >= span.start {
              break Some(tk);
            } else {
              break None;
            }
          } else {
            current_token_index += 1;
          }
        }
      } else { None };

      // derive the base color and attribute for the char
      // from the token list if available
      let mut attr = Attr::Reset;
      let mut color = DEFAULT_TRUE_COLOR;

      // check if we are in the primary location of the error,
      // else check if we are in a token, find char style overrides for the token
      if det.primary_location.contains(&char_index) {
        (color, attr) = (fg, Attr::Bold);
      } else if let Some(tk) = current_token {
        (color, attr) = char_styles_for_token(tk);
      }

      // darken color as column distance increases
      let h_dist_factor: f32 = -0.3 * (h_distance as f32 / HORIZONTAL_WINDOW_EXPANSION as f32);
      color = color.multiply_scalar(h_dist_factor.exp());

      if (color, attr) != last_formatting {
        write!(writer, "\x1B[0m{}{}", attr, color)?;
      }

      write!(writer, "{}", char)?;

      last_formatting = (color, attr);
    }

    writeln!(writer)?;
  }

  // print bottom line, which may have some highlighting
  if let Some(highlight) = last_underline_columns {
    write!(writer, concat!(
      "{}│",
      ansi!(gray),
      "      ╰─{}"
    ), fg, &rect_border_str[0..highlight.start * H_BORDER_CHAR.len_utf8()])?;
    write!(writer, "{}{}", fg, &rect_border_str[highlight.start * H_BORDER_CHAR.len_utf8()..highlight.end * H_BORDER_CHAR.len_utf8()])?;
    writeln!(writer, concat!(ansi!(gray), "{}"), &rect_border_str[highlight.end * H_BORDER_CHAR.len_utf8()..rect_border_str.len()])?;
  } else {
    writeln!(writer, concat!(
      "{}│",
      ansi!(gray),
      "      ╰─{}"
    ), fg, rect_border_str)?;
  }

  // print error header/description
  let icon = det.ty.icon();
  let prefix = det.ty.prefix();
  writeln!(writer, concat!(
    ansi!(bold), "{}{} {}",
    ansi!(_bold), "{0}{}",
    ansi!(gray), " • ",
    ansi!(bold), "{0}{}{:0>4}",
    ansi!(reset)
  ), fg, icon, prefix, det.msg, det.errno.domain, det.errno.numeral)?;

  writer.flush()
}

pub fn char_styles_for_token(tk: &Token) -> (Color, Attr) {
  if tk.ty.is_keyword() {
    return (Color::lit(0x9d39cc), Attr::Reset);
  }

  match tk.ty {
    TokenType::Error(_, _) => (Color::Red, Attr::Faint),

    TokenType::BoolLiteral(_) => (Color::lit(0xb610e8), Attr::Italic),
    TokenType::CharLiteral(_) => (Color::lit(0x7cf06e), Attr::Reset),
    TokenType::StringLiteral(_) => (Color::lit(0x61db53), Attr::Reset),
    TokenType::NumericLiteral(_, _) => (Color::lit(0xd19343), Attr::Reset),

    TokenType::Reference | TokenType::Dereference => (Color::lit(0xd6621e), Attr::Reset),
    TokenType::EvalIfPresent | TokenType::UnwrapOrReturn => (Color::lit(0xd69f1e), Attr::Reset),
    TokenType::Clone => (Color::lit(0x1e96d6), Attr::Reset),

    TokenType::Apostrophe => (Color::lit(0x1e96d6), Attr::Reset),

    _ => (DEFAULT_TRUE_COLOR, Attr::Reset)
  }
}

impl DiagnosticType {
  pub fn color(&self) -> Color {
    match self {
      DiagnosticType::Error => Color::Red,
      DiagnosticType::Warn  => Color::Yellow,
      DiagnosticType::Note  => Color::Blue,
    }
  }

  pub fn icon(&self) -> char {
    match self {
      DiagnosticType::Error => '■',
      DiagnosticType::Warn  => '▲',
      DiagnosticType::Note  => '◆',
    }
  }

  pub fn prefix(&self) -> &'static str {
    match self {
      DiagnosticType::Error => "error: ",
      DiagnosticType::Warn  => " warn: ",
      DiagnosticType::Note  => " note: ",
    }
  }
}