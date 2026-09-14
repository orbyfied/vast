use std::path::Path;
use clap::Parser as _;
use vastc_impl::diagnostics::{print_diagnostic, UnitDiagnosticContext, DiagnosticDetails, DiagnosticType, GenericErrno};
use vastc_impl::parser::lexer::Lexer;
use vastc_impl::parser::parser::{Parser, TokenCursor};
use vastc_impl::parser::token::{TokenType};
use vastc_supplemental::{debug};
use vastc_supplemental::logging::stderr_writer;
use vastc_supplemental::source::{Source};

/// The command line options which may be passed
#[derive(clap::Parser, Debug)]
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

  // load source file
  let source = Source::load_from_file(Path::new("./test.vs")).unwrap();
  debug!("Loaded source file {:?}", source);

  // create lexer state and tokenize
  let mut lexer = Lexer::new(&source);
  lexer.tokenize();
  debug!("Completed lexical analysis tokenCount: {}, errorCount: {}", lexer.tokens().len(), lexer.error_count());

  // prepare diagnostics context for first errors
  let mut diagnostic_context = UnitDiagnosticContext::new(&source);
  diagnostic_context.with_lexer(&lexer);

  // print lexer diagnostics if present
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

    print_diagnostic(&mut *writer, &diagnostic_context, &DiagnosticDetails {
      ty: DiagnosticType::Warn,
      primary_location: 2..5,
      errno: GenericErrno {
        domain: "H",
        numeral: 1234 as u32
      },

      msg: "testing warnings"
    }).unwrap();

    println!();
  }

  // trade lexer instance for token list,
  // create token stream and parse source unit
  let mut tokens = lexer.take_tokens();
  let token_cursor = TokenCursor::new(&mut tokens);
  let parser = Parser::new(token_cursor);
}