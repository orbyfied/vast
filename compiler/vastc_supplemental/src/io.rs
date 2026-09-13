use std::io;

pub fn write_repeated_char(w: &mut impl io::Write, c: char, n: usize) -> io::Result<()> {
  let s: String = std::iter::repeat(c).take(n).collect();
  w.write_all(s.as_bytes())
}