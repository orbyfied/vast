pub mod source;
pub mod logging;
pub mod parse;

/// ANSI command sequence to reset all formatting
pub const RESET: &'static str = "\x1B[0;m";