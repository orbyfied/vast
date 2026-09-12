use std::cell::LazyCell;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::Range;
use vastc_supplemental::parse::CharTree;
use vastc_supplemental::source::Segment;

/// Represents a source token with a type, optionally a value, and an attached
/// position.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
  /// The variant of this token
  pub ty: TokenType,
  /// The location in the source code
  pub location: Option<Range<u32>>,
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
  pub const TAIL_NEWLINE: u32 = 0b01;
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

  pub fn location(&self) -> &Option<Range<u32>> {
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

  StringLiteral(/* must own, may be escaped */ String),
  CharLiteral(char),
  NumericLiteral /* parsed from source span later in parsing/type resolution */,

  Identifier /* parsed from source span */,

  /*
       Keywords
   */
  Use,
  Fn,
  Let,
  Mut,

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

  Colon,       // :
  Semicolon,   // ;

  DoubleColon, // ::
  Dot,         // .
  Comma,       // ,
  Asterisk,    // *
  At,          // @


}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Errno {
  IllegalCharacter,
  UnterminatedLiteral,
  IllegalEscapedChar,
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

  tree.insert_str(":", TokenType::Colon);
  tree.insert_str(";", TokenType::Semicolon);

  tree.insert_str(".", TokenType::Dot);
  tree.insert_str(",", TokenType::Comma);
  tree.insert_str("*", TokenType::Asterisk);
  tree.insert_str("@", TokenType::At);

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

  map
});