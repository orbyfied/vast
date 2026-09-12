use vastc_supplemental::parse::CharIterator;

pub fn resolve_escaped_char(it: &mut impl CharIterator) -> Option<char> {
  Some(match it.peek_and_advance()? {
    // trivial
    'n' => '\n',
    'r' => '\r',
    't' => '\t',
    '\\' => '\\',

    // todo

    // invalid escape seq
    _ => return None
  })
}