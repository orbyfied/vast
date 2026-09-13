use std::cmp::max_by_key;
use std::path::Path;
use clap::Parser;
use vastc_impl::diagnostics::{print_diagnostic, UnitDiagnosticContext, DiagnosticDetails, DiagnosticType, GenericErrno};
use vastc_impl::parser::lexer::Lexer;
use vastc_impl::parser::token::{Token, TokenFlags, TokenType};
use vastc_supplemental::{debug};
use vastc_supplemental::logging::stderr_writer;
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
  debug!("Loaded source file {:?}", source);

  let mut lexer = Lexer::new(&source);
  lexer.tokenize();
  debug!("Completed lexical analysis tokenCount: {}, errorCount: {}", lexer.tokens().len(), lexer.error_count());

  let mut diagnostic_context = UnitDiagnosticContext::new(&source);
  diagnostic_context.with_lexer(&lexer);

  if lexer.error_count() > 0 {
    let mut writer = stderr_writer().lock().unwrap();
    lexer.errors().for_each(|err| {
      let TokenType::Error(errno, msg) = err.ty else {
        return
      };

      print_diagnostic(&mut *writer, &diagnostic_context, &DiagnosticDetails {
        ty: DiagnosticType::Error,
        primary_location: err.location.clone().unwrap(),
        errno: GenericErrno {
          domain: "T",
          numeral: errno as u32
        },

        msg
      }).unwrap();
    });

    println!();
  }

  let tokens = lexer.tokens();
  // debug!("\nTokens: {:#?}", tokens);
}