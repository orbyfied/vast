pub mod source;
pub mod logging;
pub mod parse;
pub mod ansi;
pub mod io;

/// ANSI command sequence to reset all formatting
pub const RESET: &'static str = "\x1B[0;m";