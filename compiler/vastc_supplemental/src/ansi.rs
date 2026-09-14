use std::fmt::{Display, Write};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FormatTarget {
  Foreground,
  Background,
  None
}

/// Common trait for all ANSI formatting properties.
pub trait AnsiProperty: Display {
  fn print_ansi(&self, target: FormatTarget, str: &mut impl Write) -> std::fmt::Result;
  fn off(&self) -> Self;

  fn to_ansi(&self) -> String {
    let mut s = String::new();
    self.print_ansi(FormatTarget::None, &mut s).unwrap();
    s
  }
}

macro_rules! auto_impl_display {
    ($($t:ty),* $(,)?) => {
        $(
            impl std::fmt::Display for $t {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    self.print_ansi(FormatTarget::None, f)
                }
            }
        )*
    };
}

/// Possible colorings within the terminal
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Color {
  True(u8, u8, u8),
  Fixed(u16),

  Black = 30,
  Red = 31,
  Green = 32,
  Yellow = 33,
  Blue = 34,
  Magenta = 35,
  Cyan = 36,
  White = 37,

  Gray = 90,
  BrightRed = 91,
  BrightGreen = 92,
  BrightYellow = 93,
  BrightBlue = 94,
  BrightMagenta = 95,
  BrightCyan = 96,
  BrightWhite = 97,

  Reset = 39,
}

auto_impl_display!(Color);

impl Color {
  pub const fn lit(unit: u32) -> Color {
    let r = ((unit >> 16) & 0xFF) as u8;
    let g = ((unit >> 8) & 0xFF) as u8;
    let b = (unit & 0xFF) as u8;

    Color::True(r, g, b)
  }

  pub const fn rgb_frac(r: f32, g: f32, b: f32) -> Color {
    Color::True((r * 255f32) as u8, (g * 255f32) as u8, (b * 255f32) as u8)
  }

  fn discriminant(&self) -> u8 {
    // SAFETY: `Color` is repr(u8), so it's guaranteed to start
    // with a u8 tag as its first field in memory.
    unsafe { *(self as *const Self as *const u8) }
  }

  pub fn bg(&self) -> BackgroundColor {
    BackgroundColor { inner: self.clone() }
  }

  pub fn try_to_true(&self) -> Color {
    match self {
      Color::True(_, _, _) => *self,
      // todo: idk how id do this
      _ => *self
    }
  }

  pub fn multiply_scalar(&self, factor: f32) -> Color {
    let Color::True(r, g, b) = self.try_to_true() else {
      return *self
    };

    Color::rgb_frac((r as f32 / 255.0) * factor, (g as f32 / 255.0) * factor, (b as f32 / 255.0) * factor)
  }
}

impl AnsiProperty for Color {
  fn print_ansi(&self, target: FormatTarget, str: &mut impl Write) -> std::fmt::Result {
    let nr_scoped: u8 = match target {
      FormatTarget::Foreground | FormatTarget::None => self.discriminant(),
      FormatTarget::Background => self.discriminant() + 10,
    };

    match self {
      Color::Fixed(id) => write!(str, "\x1B[38;5;{}m", id),
      Color::True(r, g, b) => write!(str, "\x1B[38;2;{};{};{}m", r, g, b),
      _ => write!(str, "\x1B[{}m", nr_scoped),
    }
  }

  fn off(&self) -> Self {
    Color::Reset
  }
}

/// Possible attributes to be applied within the terminal
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Attr {
  /// Reset *all* formatting from this point. ANSI command 0.
  Reset = 0,

  Bold = 1,
  Faint = 2,
  Italic = 3,
  Underline = 4,
  Blink = 5,
  Strike = 9,

  NormalIntensity = 22, // ~Bold or ~Faint
  NoItalic = 23,
  NoUnderline = 24,
  NoBlink = 25,
  NoStrike = 29,
}

auto_impl_display!(Attr);

impl AnsiProperty for Attr {
  fn print_ansi(&self, _target: FormatTarget, str: &mut impl Write) -> std::fmt::Result {
    write!(str, "\x1B[{}m", *self as u32)
  }

  fn off(&self) -> Self {
    match self {
      Attr::Bold => Attr::NormalIntensity,
      Attr::Faint => Attr::NormalIntensity,
      Attr::Italic => Attr::NoItalic,
      Attr::Underline => Attr::NoUnderline,
      Attr::Strike => Attr::NoStrike,
      Attr::Blink => Attr::NoBlink,
      _ => Attr::Reset,
    }
  }
}

pub struct BackgroundColor {
  inner: Color
}

auto_impl_display!(BackgroundColor);

impl AnsiProperty for BackgroundColor {
  fn print_ansi(&self, target: FormatTarget, str: &mut impl Write) -> std::fmt::Result {
    self.inner.print_ansi(target, str)
  }

  fn off(&self) -> Self {
    BackgroundColor { inner: Color::Reset }
  }
}