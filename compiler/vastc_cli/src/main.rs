use std::cmp::max_by_key;
use std::path::Path;
use clap::Parser;
use vastc_impl::parser::lexer::Lexer;
use vastc_impl::parser::token::{Token, TokenFlags};
use vastc_supplemental::{debug};
use vastc_supplemental::source::{LineIndexOps, Source};

/// The command line options which may be passed
#[derive(Parser, Debug)]
struct Opts {
  /// The source files to be included in compilation, these are considered the projects
  /// primary sources. Missing `arg` attribute implies positional.
  sources: Vec<String>,

  /// The name of the module being compiled (not the package)
  #[arg(long, default_value = "main")]
  module_name: String,
}

fn main() {
  println!();

  // Parse command line options
  let opts = Opts::parse();
  debug!("Parsed command line opts");

  ///////////////////////
  ///////////////////////

  let source = Source::load_from_file(Path::new("./test.vs")).unwrap();
  let mut lexer = Lexer::new(&source);
  lexer.tokenize();
  if lexer.error_count() > 0 {
    lexer.errors().for_each(|err| {
      println!("{}", err);
    });
  }

  let tokens = lexer.tokens();
}