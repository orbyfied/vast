use crate::ast::Ast;
use crate::diagnostics::DiagnosticDetails;
use crate::parser::token::Token;

pub struct Parser<'unit> {
  /// The current cursor over the token list
  cursor: TokenCursor<'unit>,
  /// List of produced error diagnostics
  diagnostics: Vec<DiagnosticDetails>,
  /// The managing AST object, which contains a flattened list of nodes
  /// and an optional root node reference.
  ast: Ast,
}

impl<'unit> Parser<'unit> {
  pub fn new(cursor: TokenCursor<'unit>) -> Self {
    Self {
      cursor,
      diagnostics: Vec::new(),
      ast: Ast::empty(),
    }
  }

  pub fn ast(&self) -> &Ast {
    &self.ast
  }

  pub fn diagnostics(&self) -> &Vec<DiagnosticDetails> {
    &self.diagnostics
  }
}

/// Cursor-like iteration utility over an in-memory list of tokens.
pub struct TokenCursor<'a> {
  tokens: &'a mut Vec<Token>,
  index: usize
}

impl<'a> TokenCursor<'a> {
  pub fn new(tokens: &'a mut Vec<Token>) -> Self {
    TokenCursor {
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