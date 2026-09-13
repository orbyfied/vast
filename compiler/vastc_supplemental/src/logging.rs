use std::cell::{Cell, UnsafeCell};
use std::io;
use std::io::{BufWriter, Stderr, Write};
use std::ops::Add;
use std::sync::{LazyLock, Mutex, OnceLock, RwLock};

/// The shared buffered writer used by all logging macros
static STDERR_WRITER: OnceLock<Mutex<BufWriter<Stderr>>> = OnceLock::new();

pub fn stderr_writer() -> &'static Mutex<BufWriter<Stderr>> {
  STDERR_WRITER.get_or_init(|| Mutex::new(BufWriter::new(std::io::stderr())))
}

/// The maximum log level, set once by the program
pub static MAX_LOG_LEVEL: OnceLock<i32> = OnceLock::new();

/// Default if not reconfigured by initialization
pub const DEFAULT_MAX_LOG_LEVEL: i32 = 99;

/// Define general log() function by macro rules
#[macro_export] macro_rules! log {
    ($level:ident /* Level:: */ /* format + args */ $(,$arg:expr)*) => {
      if ($crate::logging::MAX_LOG_LEVEL.get().copied().unwrap_or($crate::logging::DEFAULT_MAX_LOG_LEVEL) > 4) {
        use std::io::Write;
        let mut writer = $crate::logging::stderr_writer().lock().unwrap();

        write!(writer, " {}", $crate::level_prefix!($level)).expect("vastc_logging: write");
        write!(writer, " \x1B[38;2;95;116;133m[\x1B[38;2;119;136;153m{}\x1B[38;2;95;116;133m:\x1B[38;2;119;136;153m{}\x1B[38;2;95;116;133m]", file!(), line!()).expect("vastc_logging: write");
        write!(writer, " \x1B[0m\x1B[1;38;2;146;160;173m{} • ", module_path!()).expect("vastc_logging: write");
        write!(writer, "\x1B[0m\x1B[97m").expect("vastc_logging: write");

        // finally, print message + newline and flush
        write!(writer $(,$arg)*).expect("vastc_logging: write");
        writer.write("\x1B[0m\n".as_bytes()).expect("vastc_logging: raw write");
        writer.flush().expect("vastc_logging: flush of stderr buffer failed");
      }
    };
}

/* 
    Statically compiled ANSI formatting macros
 */

#[macro_export] macro_rules! trace { ($fmt:expr $(,$arg:expr)*) => { $crate::log!(trace, $fmt $(,$arg)*) }; }
#[macro_export] macro_rules! debug { ($fmt:expr $(,$arg:expr)*) => { $crate::log!(debug, $fmt $(,$arg)*) }; }
#[macro_export] macro_rules! ok { ($fmt:expr $(,$arg:expr)*) => { $crate::log!(ok, $fmt $(,$arg)*) }; }
#[macro_export] macro_rules! info { ($fmt:expr $(,$arg:expr)*) => { $crate::log!(info, $fmt $(,$arg)*) }; }
#[macro_export] macro_rules! warn { ($fmt:expr $(,$arg:expr)*) => { $crate::log!(warn, $fmt $(,$arg)*) }; }
#[macro_export] macro_rules! err { ($fmt:expr $(,$arg:expr)*) => { $crate::log!(error, $fmt $(,$arg)*) }; }
#[macro_export] macro_rules! fatal { ($fmt:expr $(,$arg:expr)*) => { $crate::log!(fatal, $fmt $(,$arg)*) }; }

#[macro_export] macro_rules! level_prefix {
  (trace) => { $crate::ansi_fmt!("Trace", purple, bold) };
  (debug) => { $crate::ansi_fmt!("Debug", blue, bold) };
  (ok) =>    { $crate::ansi_fmt!("   Ok", green, bold) };
  (info) =>  { $crate::ansi_fmt!(" Info", cyan, bold) };
  (warn) =>  { $crate::ansi_fmt!(" Warn", yellow, bold) };
  (error) => { $crate::ansi_fmt!("Error", red, bold) };
  (fatal) => { $crate::ansi_fmt!("Fatal", red, bold) }
}

#[macro_export] macro_rules! level_ord {
  (trace) => { 7 };
  (debug) => { 6 };
  (ok) =>    { 5 };
  (info) =>  { 4 };
  (warn) =>  { 3 };
  (error) => { 2 };
  (fatal) => { 1 }
}

/// Surround the given message with ANSI formatting
#[macro_export] macro_rules! ansi_fmt {
   ($msg:literal, $first:tt $(,$rest:tt)*) => {
     concat!($crate::ansi!($first $(,$rest)*), $msg, "\x1B[0m")
   };
}

/// Escape the given ANSI command code and continue with the `rest` parameters
#[macro_export] macro_rules! esc {
  ($a:expr $(,$rest:tt)*) => { concat!("\x1B[", $a, "m" $(, $crate::ansi!($rest))* ) };
}

/// Expands to the ANSI escaped command sequence for the given set of formatting rules
#[macro_export] macro_rules! ansi {
  (reset $(,$rest:tt)*) => { $crate::esc!(0 $(,$rest)*) };
  (_ $(,$rest:tt)*) => { $crate::esc!(0 $(,$rest)*) };
  (0 $(,$rest:tt)*) => { $crate::esc!(0 $(,$rest)*) };

  (bold $(,$rest:tt)*) => { $crate::esc!(1 $(,$rest)*) };
  (faint $(,$rest:tt)*) => { $crate::esc!(2 $(,$rest)*) };
  (_bold $(,$rest:tt)*) => { $crate::esc!(22 $(,$rest)*) };
  (_faint $(,$rest:tt)*) => { $crate::esc!(22 $(,$rest)*) };
  (italic $(,$rest:tt)*) => { $crate::esc!(3 $(,$rest)*) };
  (_italic $(,$rest:tt)*) => { $crate::esc!(23 $(,$rest)*) };
  (underline $(,$rest:tt)*) => { $crate::esc!(4 $(,$rest)*) };
  (_underline $(,$rest:tt)*) => { $crate::esc!(24 $(,$rest)*) };
  (blink $(,$rest:tt)*) => { $crate::esc!(5 $(,$rest)*) };
  (_blink $(,$rest:tt)*) => { $crate::esc!(25 $(,$rest)*) };

  (nfg $(,$rest:tt)*) => { $crate::esc!(39 $(,$rest)*) };
  (nbg $(,$rest:tt)*) => { $crate::esc!(49 $(,$rest)*) };

  // foreground colors //
  (black $(,$rest:tt)*) => { $crate::esc!(30 $(,$rest)*) };
  (red $(,$rest:tt)*) => { $crate::esc!(31 $(,$rest)*) };
  (green $(,$rest:tt)*) => { $crate::esc!(32 $(,$rest)*) };
  (yellow $(,$rest:tt)*) => { $crate::esc!(33 $(,$rest)*) };
  (blue $(,$rest:tt)*) => { $crate::esc!(34 $(,$rest)*) };
  (purple $(,$rest:tt)*) => { $crate::esc!(35 $(,$rest)*) };
  (cyan $(,$rest:tt)*) => { $crate::esc!(36 $(,$rest)*) };
  (white $(,$rest:tt)*) => { $crate::esc!(37 $(,$rest)*) };
  (gray $(,$rest:tt)*) => { $crate::esc!(90 $(,$rest)*) };
  (bred $(,$rest:tt)*) => { $crate::esc!(91 $(,$rest)*) };
  (bgreen $(,$rest:tt)*) => { $crate::esc!(92 $(,$rest)*) };
  (byellow $(,$rest:tt)*) => { $crate::esc!(93 $(,$rest)*) };
  (bblue $(,$rest:tt)*) => { $crate::esc!(94 $(,$rest)*) };
  (bpurple $(,$rest:tt)*) => { $crate::esc!(95 $(,$rest)*) };
  (bcyan $(,$rest:tt)*) => { $crate::esc!(96 $(,$rest)*) };
  (bwhite $(,$rest:tt)*) => { $crate::esc!(97 $(,$rest)*) };
  (fixed($n:literal) $(,$rest:tt)*) => { $crate::esc!(concat!("38;5;", $n) $(,$rest)*) };
  (true($r:literal, $g:literal, $b:literal) $(,$rest:tt)*) => { $crate::esc!(concat!("38;2;", $r, ";", $g, ";", $b) $(,$rest)*) };

  // background colors //
  (bg($color:tt) $(,$rest:tt)*) => { $crate::esc!($crate::to_bg_code!($color) $(,$rest)*) };
  (bg(fixed($n:literal)) $(,$rest:tt)*) => { $crate::esc!($crate::to_bg_code!(fixed($n)) $(,$rest)*) };
  (bg(true($r:literal, $g:literal, $b:literal)) $(,$rest:tt)*) => { $crate::esc!($crate::to_bg_code!(true($r, $g, $b)) $(,$rest)*) };
}

/// Internal macro to convert an item expression to its background code string fragment
#[doc(hidden)]
#[macro_export] macro_rules! to_bg_code {
  (black) => { 40 };
  (red) => { 41 };
  (green) => { 42 };
  (yellow) => { 43 };
  (blue) => { 44 };
  (purple) => { 45 };
  (cyan) => { 46 };
  (white) => { 47 };
  (gray) => { 100 };
  (bred) => { 101 };
  (bgreen) => { 102 };
  (byellow) => { 103 };
  (bblue) => { 104 };
  (bpurple) => { 105 };
  (bcyan) => { 106 };
  (bwhite) => { 107 };
  (fixed($n:literal)) => { concat!("48;5;", $n) };
  (true($r:literal, $g:literal, $b:literal)) => { concat!("48;2;", $r, ";", $g, ";", $b) };
}