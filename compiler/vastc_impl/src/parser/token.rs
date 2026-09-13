use std::cell::LazyCell;
use std::cmp::{max, min};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::Range;
use vastc_supplemental::ansi;
use vastc_supplemental::parse::CharTree;
use vastc_supplemental::source::{Segment, Source, SourceIndex, SourceSpan, SourceSpanOps};

/// Represents a source token with a type, optionally a value, and an attached
/// position.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
  /// The variant of this token
  pub ty: TokenType,
  /// The location in the source code
  pub location: Option<SourceSpan>,
  /// Additional flags
  pub flags: u32
}

impl Display for Token {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_fmt(format_args!("Token(ty: {:?}, flags: {}, loc: {:?})", self.ty, self.flags, self.location))
  }
}

pub struct TokenFlags;

impl TokenFlags {
  /// Denotes this token was followed directly by a space or a tab
  pub const TAIL_SPACE: u32 = 0b1;
  /// Denotes this token was followed directly by a newline
  pub const TAIL_NEWLINE: u32 = 0b10;
}

impl Token {
  pub const EOF: Token = Token {
    location: None,
    ty: TokenType::EOF,
    flags: 0
  };

  pub fn ty(&self) -> &TokenType {
    &self.ty
  }

  pub fn location(&self) -> &Option<SourceSpan> {
    &self.location
  }

  pub fn is_error(&self) -> bool {
    self.ty.is_error()
  }

  pub fn has_flags(&self, mask: u32) -> bool {
    !(self.flags & mask) == 0
  }
}

/// Cursor-like iteration utility over an in-memory list of tokens.
pub struct TokenStream<'a> {
  tokens: &'a Vec<Token>,
  index: usize
}

impl<'a> TokenStream<'a> {
  pub fn new(tokens: &'a Vec<Token>) -> Self {
    TokenStream {
      tokens,
      index: 0
    }
  }

  pub fn at(&self, idx: usize) -> &Token {
    if idx >= self.tokens.len() {
      &self.tokens[idx]
    } else {
      &Token::EOF
    }
  }

  pub fn advance(&mut self) -> &Token {
    self.index += 1;
    self.at(self.index)
  }

  pub fn peek(&self) -> &Token {
    self.at(self.index)
  }
}

/// The premier token type and associated value if applicable as a
/// tagged union/enum.
#[derive(Clone, Debug, PartialEq)]
pub enum TokenType {
  /*
       Internal
   */
  /// Marks an error in tokenization.
  Error(Errno, &'static str),
  /// The token which marks the end of the file. This is always the last token in the list.
  /// Also returned for out-of-bounds peek operations.
  EOF,

  /*
       Literals
   */

  BoolLiteral(bool),
  StringLiteral(/* must own, may be escaped */ String),
  CharLiteral(char),
  NumericLiteral(SourceSpan, NumericLiteralMetadata) /* parsed from source span later in parsing/type resolution */,

  Identifier /* parsed from source span */,

  /*
       Keywords
   */
  Use,
  Fn,
  Let,
  Mut,
  If,
  Else,
  Match,
  Struct,
  Trait,
  Dyn,
  Impl,

  Public,
  LSelf, // Lower Self -- "self"
  USelf, // Upper Self -- "Self"

  /*
       Symbols
   */
  LParen,      // (
  RParen,      // )
  LBrace,      // {
  RBrace,      // }
  LBracket,    // [
  RBracket,    // ]
  LAngle,      // <
  RAngle,      // >

  Colon,       // :
  Semicolon,   // ;

  DoubleColon, // ::
  Dot,         // .
  Comma,       // ,
  At,          // @

  /*
      Assignment
   */
  Assign,      // =

  IncrBy,
  DecrBy,
  MulBy,
  DivBy,
  ModBy,

  AndBy,
  OrBy,
  ShlBy,
  ShrBy,
  XorBy,

  /*
      Operators
   */

  /* Arithmetic */
  Plus,    // +
  Minus,   // -
  Times,   // *
  Divide,  // /
  Modulo,  // %

  /* Logic/Binary */
  And,    // &
  Or,     // |
  Invert, // ~
  Not,    // !
  And2,   // &&
  Or2,    // ||
  Shl,    // <<
  Shr,    // >>
  Xor,    // ^

  /* Comparison */
  Eq,      // ==
  Neq,     // !=
  Gt,      // >
  GtOrEq,  // >=
  Lt,      // <
  LtOrEq,  // <=

}

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Errno {
  IllegalCharacter = 1,
  UnterminatedLiteral,
  IllegalEscapedChar,
  IllegalRadixChar,
  IllegalDecimalRadix,
  InvalidTypeQualifier,
}

impl TokenType {
  pub fn is_error(&self) -> bool {
    match self {
      TokenType::Error(_, _) => true,
      _ => false
    }
  }
}

/// Parsing character tree for all symbol tokens, evaluated before identifier names.
/// This includes special characters, indexing, operators, etc.
pub const TREE_ALL_SYMBOLS: LazyCell<CharTree<TokenType>> = LazyCell::new(|| {
  let mut tree: CharTree<TokenType> = CharTree::new();

  tree.insert_str("(", TokenType::LParen);
  tree.insert_str(")", TokenType::RParen);
  tree.insert_str("{", TokenType::LBrace);
  tree.insert_str("}", TokenType::RBrace);
  tree.insert_str("[", TokenType::LBracket);
  tree.insert_str("]", TokenType::RBracket);
  tree.insert_str("<", TokenType::LAngle);
  tree.insert_str(">", TokenType::RAngle);

  tree.insert_str(":", TokenType::Colon);
  tree.insert_str(";", TokenType::Semicolon);

  tree.insert_str(".", TokenType::Dot);
  tree.insert_str(",", TokenType::Comma);
  tree.insert_str("@", TokenType::At);
  tree.insert_str("::", TokenType::DoubleColon);

  tree.insert_str("=", TokenType::Assign);

  tree.insert_str("+=", TokenType::IncrBy);
  tree.insert_str("-=", TokenType::DecrBy);
  tree.insert_str("*=", TokenType::MulBy);
  tree.insert_str("/=", TokenType::DivBy);
  tree.insert_str("%=", TokenType::ModBy);

  tree.insert_str("&=", TokenType::AndBy);
  tree.insert_str("|=", TokenType::OrBy);
  tree.insert_str("<<=", TokenType::ShlBy);
  tree.insert_str(">>=", TokenType::ShrBy);
  tree.insert_str("^=", TokenType::XorBy);

  tree.insert_str("+", TokenType::Plus);
  tree.insert_str("-", TokenType::Minus);
  tree.insert_str("*", TokenType::Times);
  tree.insert_str("/", TokenType::Divide);
  tree.insert_str("%", TokenType::Modulo);

  tree.insert_str("&", TokenType::And);
  tree.insert_str("&&", TokenType::And2);
  tree.insert_str("|", TokenType::Or);
  tree.insert_str("||", TokenType::Or2);
  tree.insert_str("!", TokenType::Not);
  tree.insert_str("~", TokenType::Invert);
  tree.insert_str(">>", TokenType::Shr);
  tree.insert_str("<<", TokenType::Shl);
  tree.insert_str("^", TokenType::Xor);

  tree.insert_str("==", TokenType::Eq);
  tree.insert_str("!=", TokenType::Neq);
  tree.insert_str(">", TokenType::Gt);
  tree.insert_str(">=", TokenType::GtOrEq);
  tree.insert_str("<", TokenType::Lt);
  tree.insert_str("<=", TokenType::LtOrEq);

  tree
});

/// Map of (static lifetime) string literals to keyword token type
pub const KEYWORD_MAP: LazyCell<HashMap<&'static str, TokenType>> = LazyCell::new(|| {
  let mut map: HashMap<&'static str, TokenType> = HashMap::new();

  map.insert("fn", TokenType::Fn);
  map.insert("public", TokenType::Public);
  map.insert("let", TokenType::Let);
  map.insert("mut", TokenType::Mut);
  map.insert("use", TokenType::Use);
  map.insert("self", TokenType::LSelf);
  map.insert("Self", TokenType::USelf);
  map.insert("struct", TokenType::Struct);
  map.insert("impl", TokenType::Impl);
  map.insert("if", TokenType::If);
  map.insert("else", TokenType::Else);
  map.insert("match", TokenType::Match);
  map.insert("dyn", TokenType::Dyn);

  map.insert("true", TokenType::BoolLiteral(true));
  map.insert("false", TokenType::BoolLiteral(false));

  map
});

#[derive(Clone, Debug, PartialEq)]
pub enum NumericTypeQualifier {
  Infer,
  Float(/* width */ u32),
  Int(/* width */ u32),
  Unsigned(/* width */ u32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct NumericLiteralMetadata {
  pub is_float: bool,                  // whether this literal should be interpreted as a decimal (`.` or F/D suffix)
  pub radix: u16,                      // used later to reparse actual number literal content
  pub type_qual: NumericTypeQualifier, // type qualifier if explicitly specified, otherwise infer
}

impl TokenType {
  pub fn is_keyword(&self) -> bool {
    match self {
      TokenType::Use |
      TokenType::Let | TokenType::Fn | TokenType::USelf | TokenType::LSelf | TokenType::Public | TokenType::Mut
      => true,

      _ => false,
    }
  }
}