use std::collections::HashMap;
use std::ops::Range;

/// An iterator of a sized character sequence.
pub trait CharIterator {
  fn index(&self) -> u32;
  fn restore(&mut self, idx: u32);
  fn advance_0(&mut self) -> Option<char>;
  fn peek_0(&self) -> Option<char>;
  fn done(&self) -> bool;
  
  fn peek_and_advance(&mut self) -> Option<char> {
    let ch = self.peek_0();
    self.advance_0();
    ch
  }
}

/// A character tree (node) used to parse aliased tokens from strings.
pub struct CharTree<T: Clone> {
  associate: Option<T>,
  children: HashMap<char, CharTree<T>>
}

impl<T: Clone> CharTree<T> {
  pub fn new() -> CharTree<T> {
    CharTree {
      associate: None,
      children: HashMap::new()
    }
  }

  pub fn insert_str(&mut self, chars: &str, associate: T) -> &mut Self {
    self.insert(chars.as_bytes(), associate)
  }

  pub fn insert(&mut self, chars: &[u8], associate: T) -> &mut Self {
    assert!(chars.len() > 0);
    let mut curr: &mut CharTree<T> = self;
    for i in 0..chars.len() {
      curr = curr.children.entry(chars[i] as char).or_insert_with(|| CharTree::new());
    }

    curr.associate = Some(associate);
    self
  }

  pub fn read_or_restore(&self, it: &mut impl CharIterator) -> Option<(/* parsed associate */ T, /* range where it was parsed */ Range<u32>)> {
    let start = it.index();
    let mut current: &CharTree<T> = self;
    while !it.done() {
      let c = it.peek_0().unwrap();

      // try find child for the given char
      current = match current.children.get(&c) {
        Some(child) => child,
        None => {
          it.restore(start);
          return None
        }
      };

      it.advance_0();
    }

    match &current.associate {
      None => {
        it.restore(start);
        None
      },

      Some(a) => Some((a.clone(), start..it.index()))
    }
  }
}