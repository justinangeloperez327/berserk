use std::{
    iter::FromIterator,
    ops::{Deref, DerefMut},
    slice,
};

/// Claw's typed model collection.
///
/// It behaves like a Rust slice/iterator while giving the ORM a stable
/// integration point for pagination, eager loading, views, and future helpers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Collection<T> {
    items: Vec<T>,
}

impl<T> Collection<T> {
    pub const fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn from_vec(items: Vec<T>) -> Self {
        Self { items }
    }

    pub fn into_vec(self) -> Vec<T> {
        self.items
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn first(&self) -> Option<&T> {
        self.items.first()
    }

    pub fn last(&self) -> Option<&T> {
        self.items.last()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }

    pub fn iter(&self) -> slice::Iter<'_, T> {
        self.items.iter()
    }

    pub fn iter_mut(&mut self) -> slice::IterMut<'_, T> {
        self.items.iter_mut()
    }

    pub fn push(&mut self, value: T) {
        self.items.push(value);
    }

    pub fn map<U>(self, mapper: impl FnMut(T) -> U) -> Collection<U> {
        self.items.into_iter().map(mapper).collect()
    }

    pub fn filter(mut self, mut predicate: impl FnMut(&T) -> bool) -> Self {
        self.items.retain(|value| predicate(value));
        self
    }
}

impl<T> Default for Collection<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> From<Vec<T>> for Collection<T> {
    fn from(items: Vec<T>) -> Self {
        Self::from_vec(items)
    }
}

impl<T: PartialEq> PartialEq<Vec<T>> for Collection<T> {
    fn eq(&self, other: &Vec<T>) -> bool {
        self.items == *other
    }
}

impl<T> FromIterator<T> for Collection<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::from_vec(iter.into_iter().collect())
    }
}

impl<T> Deref for Collection<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl<T> DerefMut for Collection<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}

impl<T> AsRef<[T]> for Collection<T> {
    fn as_ref(&self) -> &[T] {
        &self.items
    }
}

impl<T> IntoIterator for Collection<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a Collection<T> {
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Collection<T> {
    type Item = &'a mut T;
    type IntoIter = slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter_mut()
    }
}
