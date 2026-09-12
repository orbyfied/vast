use std::{any::type_name, collections::HashMap, hash::Hash, marker::PhantomData, sync::Arc};

/// An object which may be assigned and associated by a 'name' of type N
pub trait Named<N> where N: Eq + Hash {
  fn name(&self) -> Option<&N>;
}

#[derive(Debug, PartialEq, Hash)]
pub struct Id<T> {
  /// The index into the object registry containing the object referred to by this ID
  index: u32,

  _m: PhantomData<T>
}

impl<T> Clone for Id<T> {
  fn clone(&self) -> Self {
    Self { index: self.index, _m: PhantomData }
  }
}

impl<T> Copy for Id<T> { }

impl<T> From<usize> for Id<T> {
  fn from(value: usize) -> Self {
    Self { index: value as u32, _m: PhantomData }
  }
}

/// An arena or registry which owns and identifies objects
pub trait Ids<T> {
  fn get(&self, id: Id<T>) -> &T;
  fn get_mut(&mut self, id: Id<T>) -> &mut T;
}

/// A registry of nameless objects by type T, associated by an index (Id)
pub struct Arena<T> {
  list: Vec<T>,
}

impl<T> Arena<T> {
  pub fn new() -> Self {
    Self { list: Vec::new() }
  }

  pub fn push(&mut self, obj: T) -> (Id<T>, &mut T) {
    (self.list.len().into(), self.list.push_mut(obj))
  }

  pub fn create<F>(&mut self, constructor: F) -> (Id<T>, &mut T) where F: FnOnce() -> T {
    self.push(constructor())
  }
}

impl<T> Ids<T> for Arena<T> {
  fn get(&self, id: Id<T>) -> &T {
    self.list.get(id.index as usize).expect(&format!("corrupted arena object id   {}   for type   {}", id.index, type_name::<T>()).into_boxed_str())
  }

  fn get_mut(&mut self, id: Id<T>) -> &mut T {
    self.list.get_mut(id.index as usize).expect(&format!("corrupted arena object id   {}   for type   {}", id.index, type_name::<T>()).into_boxed_str())
  }
}

/// A registry of objects by type T, associated by an index (Id) a lookup by the associated name type.
/// It is a specific case of an Arena<T> where T is named
pub struct Registry<T, N> where N: Eq + Hash + Clone, T: Named<N> {
  arena: Arena<T>,
  lookup: HashMap<N, Id<T>>
}

impl<T, N> Registry<T, N> where N: Eq + Hash + Clone, T: Named<N> {
  pub fn new() -> Self {
    Self { arena: Arena::new(), lookup: HashMap::new() }
  }

  pub fn push(&mut self, obj: T) -> (Id<T>, &mut T) {
    let (id, ret) = self.arena.push(obj);
    if let Some(name) = ret.name() {
      _ = self.lookup.insert(name.clone(), id)
    }

    (id, ret)
  }

  pub fn create_anonymous<F>(&mut self, constructor: F) -> (Id<T>, &mut T) where F: FnOnce() -> T {
    self.push(constructor())
  }

  pub fn create_named<F>(&mut self, name: N, constructor: F) -> (Id<T>, &mut T) where F: FnOnce(N) -> T {
    self.push(constructor(name))
  }
}

impl<T, N> Ids<T> for Registry<T, N> where N: Eq + Hash + Clone, T: Named<N> {
  fn get(&self, id: Id<T>) -> &T { self.arena.get(id) }
  fn get_mut(&mut self, id: Id<T>) -> &mut T { self.arena.get_mut(id) }
}