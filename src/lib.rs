//! # `index_vec`
//!
//! A more type-safe version of `Vec`, for when `usize` just isn't cutting it
//! anymore.
//!
//! ## Example / Overview
//! ```rust
//! use index_vec::{IndexVec, index_vec};
//!
//! // Define a custom index type.
//! index_vec::define_index_type! {
//!     // In this case, use a u32 instead of a usize.
//!     pub struct StrIdx = u32;
//!     // Note that this macro has a decent amount of configurability, so
//!     // be sure to read its documentation if you think it's doing
//!     // something you don't want.
//! }
//!
//! // Create a vector which can be accessed using `StrIdx`s.
//! let mut strs: IndexVec<StrIdx, &'static str> = index_vec!["strs", "bar", "baz"];
//!
//! // l is a `StrIdx`
//! let l = strs.last_idx();
//! assert_eq!(strs[l], "baz");
//!
//! let new_i = strs.push("quux");
//! assert_eq!(strs[new_i], "quux");
//!
//! // Indices are mostly interoperable with `usize`, and support
//! // a lot of what you might want to do to an index.
//!
//! // Comparison
//! assert_eq!(StrIdx::new(0), 0usize);
//! // Addition
//! assert_eq!(StrIdx::new(0) + 1, 1usize);
//!
//! // Subtraction (Note that by default, the index will panic on overflow,
//! // but that can be configured in the macro)
//! assert_eq!(StrIdx::new(1) - 1, 0usize);
//!
//! // Wrapping
//! assert_eq!(StrIdx::new(5) % strs.len(), 1usize);
//! ```
//! ## Background
//!
//! The goal is to replace the pattern of using a `type FooIdx = usize` to
//! access a `Vec<Foo>` with something that can statically prevent using a
//! `FooIdx` in a `Vec<Bar>`. It's most useful if you have a bunch of indices
//! referring to different sorts of vectors.
//!
//! Much of the code for this is taken from `rustc`'s `IndexVec` code, however
//! it's diverged a decent amount at this point. Some notable changes:
//!
//! - No usage of unstable features.
//! - Different syntax for defining index types.
//! - More complete mirroring of Vec's API.
//! - Allows use of using other index types than `u32`/`usize`.
//! - More flexible behavior around how strictly some checks are performed,
//!
//! ## Other crates
//!
//! The [`indexed_vec`](https://crates.io/crates/indexed_vec) crate predates
//! this, and is a much closer copy of the code from `rustc`. Unfortunately,
//! this means it does not compile on stable.
//!
//! If you're looking for something further from a vec and closer to a map, you
//! might find [`handy`](https://crates.io/crates/handy),
//! [`slotmap`](https://crates.io/crates/slotmap), or
//! [`slab`](https://crates.io/crates/slab) to be closer what you want.

#![no_std]
extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use core::fmt;
use core::fmt::Debug;
use core::hash::Hash;
use core::iter::{self, FromIterator};
use core::marker::PhantomData;
use core::ops::{Index, IndexMut, Range, RangeBounds};
use core::slice;
use core::u32;

#[macro_use]
mod macros;
pub use macros::*;

#[cfg(feature = "example_generated")]
pub mod example_generated;

/// Represents a wrapped value convertable to and from a `usize`.
///
/// Generally you implement this via the [`define_index_type!`] macro, rather
/// than manually implementing it.
///
/// # Overflow
///
/// `Idx` impls are allowed to be smaller than `usize`, which means converting
/// `usize` to an `Idx` implementation might have to handle overflow.
///
/// The way overflow is handled is up to the implementation of `Idx`, but it's
/// generally panicing, unless it was turned off via the `CHECK_MAX_INDEX_IF`
/// parameter in `define_index_type`.
///
/// This trait, as you can see, doesn't have a `try_from_usize`. The `IndexVec`
/// type doesn't have additional functions exposing ways to handle index
/// overflow. I'm open to adding these, but at the moment you should pick a size
/// large enough that you won't hit problems, or verify the size cannot overflow
/// elsewhere.
pub trait Idx: Copy + 'static + Ord + Debug + Hash {
    /// Roughly equivalent to From<usize>
    fn from_usize(idx: usize) -> Self;
    /// Roughly equivalent to Into<usize>
    fn index(self) -> usize;
}

impl Idx for usize {
    #[inline]
    fn from_usize(idx: usize) -> Self {
        idx
    }
    #[inline]
    fn index(self) -> usize {
        self
    }
}

impl Idx for u32 {
    #[inline]
    fn from_usize(idx: usize) -> Self {
        assert!(idx <= u32::max_value() as usize);
        idx as u32
    }

    #[inline]
    fn index(self) -> usize {
        self as usize
    }
}

/// A Vec that only accepts indices of a specific type.
///
/// This is a thin wrapper around `Vec`, to the point where the backing vec is a
/// public property. This is in part because I know this API is not a complete
/// mirror of Vec's (patches welcome). In the worst case, you can always do what
/// you need to the Vec itself.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct IndexVec<I: Idx, T> {
    /// Our wrapped Vec.
    pub vec: Vec<T>,
    _marker: PhantomData<fn(&I)>,
}

// Whether `IndexVec` is `Send` depends only on the data,
// not the phantom data.
unsafe impl<I: Idx, T> Send for IndexVec<I, T> where T: Send {}

impl<I: Idx, T: fmt::Debug> fmt::Debug for IndexVec<I, T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.vec, fmt)
    }
}


type Enumerated<Iter, I, T> = iter::Map<iter::Enumerate<Iter>, (fn((usize, T)) -> (I, T))>;

impl<I: Idx, T> IndexVec<I, T> {
    /// Construct a new IndexVec.
    #[inline]
    pub fn new() -> Self {
        IndexVec {
            vec: Vec::new(),
            _marker: PhantomData,
        }
    }

    /// Construct a `IndexVec` from a `Vec<T>`.
    ///
    /// Panics if it's length is too large for our index type.
    #[inline]
    pub fn from_vec(vec: Vec<T>) -> Self {
        // See if `I::from_usize` might be upset by this length.
        let _ = I::from_usize(vec.len());
        IndexVec {
            vec,
            _marker: PhantomData,
        }
    }

    /// Construct an IndexVec that can hold at least `capacity` items before
    /// reallocating. See [`Vec::with_capacity`].
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        IndexVec {
            vec: Vec::with_capacity(capacity),
            _marker: PhantomData,
        }
    }

    /// Get a the storage as a `&[T]`
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        self.vec.as_slice()
    }

    /// Get a the storage as a `&mut [T]`
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.vec.as_mut_slice()
    }

    /// Push a new item onto the vector, and return it's index.
    #[inline]
    pub fn push(&mut self, d: T) -> I {
        let idx = I::from_usize(self.len());
        self.vec.push(d);
        idx
    }

    /// Pops the last item off, returning it. See [`Vec::pop`].
    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        self.vec.pop()
    }

    /// Returns the length of our vector.
    #[inline]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    /// Returns true if we're empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    /// Get an iterator that moves our of our vector.
    #[inline]
    pub fn into_iter(self) -> vec::IntoIter<T> {
        self.vec.into_iter()
    }

    /// Similar to `self.into_iter().enumerate()` but with indices of `I` and
    /// not `usize`.
    #[inline]
    pub fn into_iter_enumerated(self) -> Enumerated<vec::IntoIter<T>, I, T> {
        self.vec.into_iter().enumerate().map(|(i, t)| (Idx::from_usize(i), t))
    }

    /// Get a iterator over reverences to our values.
    #[inline]
    pub fn iter(&self) -> slice::Iter<'_, T> {
        self.vec.iter()
    }

    /// Similar to `self.iter().enumerate()` but with indices of `I` and not
    /// `usize`.
    #[inline]
    pub fn iter_enumerated(&self) -> Enumerated<slice::Iter<'_, T>, I, &T> {
        self.vec.iter().enumerate().map(|(i, t)| (Idx::from_usize(i), t))
    }

    /// Get an interator over all our indices.
    #[inline]
    pub fn indices(&self) -> iter::Map<Range<usize>, fn(usize) -> I> {
        (0..self.len()).map(Idx::from_usize)
    }

    /// Get a iterator over mut reverences to our values.
    #[inline]
    pub fn iter_mut(&mut self) -> slice::IterMut<'_, T> {
        self.vec.iter_mut()
    }

    /// Similar to `self.iter_mut().enumerate()` but with indices of `I` and not
    /// `usize`.
    #[inline]
    pub fn iter_mut_enumerated(&mut self) -> Enumerated<slice::IterMut<'_, T>, I, &mut T> {
        self.vec.iter_mut().enumerate().map(|(i, t)| (Idx::from_usize(i), t))
    }

    /// Return an iterator that removes the items from the requested range. See
    /// [`Vec::drain`].
    #[inline]
    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> vec::Drain<'_, T> {
        self.vec.drain(range)
    }

    /// Similar to `self.drain(r).enumerate()` but with indices of `I` and not
    /// `usize`.
    #[inline]
    pub fn drain_enumerated<R: RangeBounds<usize>>(&mut self, range: R) -> Enumerated<vec::Drain<'_, T>, I, T> {
        self.vec.drain(range).enumerate().map(|(i, t)| (Idx::from_usize(i), t))
    }

    /// Return the index of the last element, if we are not empty.
    #[inline]
    pub fn last(&self) -> Option<I> {
        self.len().checked_sub(1).map(I::from_usize)
    }

    /// Shrinks the capacity of the vector as much as possible.
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.vec.shrink_to_fit()
    }

    /// Swaps two elements in our vector.
    #[inline]
    pub fn swap(&mut self, a: I, b: I) {
        self.vec.swap(a.index(), b.index())
    }

    /// Shortens the vector, keeping the first `len` elements and dropping
    /// the rest. See [`Vec::truncate`]
    #[inline]
    pub fn truncate(&mut self, a: usize) {
        self.vec.truncate(a)
    }

    /// Clear our vector. See [`Vec::clear`].
    #[inline]
    pub fn clear(&mut self) {
        self.vec.clear()
    }

    /// Reserve capacity for `c` more elements. See [`Vec::reserve`]
    #[inline]
    pub fn reserve(&mut self, c: usize) {
        self.vec.reserve(c)
    }

    /// Gives the next index that will be assigned when `push` is
    /// called.
    #[inline]
    pub fn next_idx(&self) -> I {
        I::from_usize(self.len())
    }

    /// Return the index of the last element, or panic.
    #[inline]
    pub fn last_idx(&self) -> I {
        assert!(!self.is_empty());
        I::from_usize(self.len() - 1)
    }

    /// Get a ref to the item at the provided index, or None for out of bounds.
    #[inline]
    pub fn get(&self, index: I) -> Option<&T> {
        self.vec.get(index.index())
    }

    /// Get a mut ref to the item at the provided index, or None for out of
    /// bounds
    #[inline]
    pub fn get_mut(&mut self, index: I) -> Option<&mut T> {
        self.vec.get_mut(index.index())
    }

    /// Resize ourselves in-place to `new_len`. See [`Vec::resize`].
    #[inline]
    pub fn resize(&mut self, new_len: usize, value: T) where T: Clone {
        self.vec.resize(new_len, value)
    }

    /// Resize ourselves in-place to `new_len`. See [`Vec::resize_with`].
    #[inline]
    pub fn resize_with<F: FnMut() -> T>(&mut self, new_len: usize, f: F) {
        self.vec.resize_with(new_len, f)
    }

    /// Moves all the elements of `other` into `Self`, leaving `other` empty.
    /// See [`Vec::append`].
    #[inline]
    pub fn append(&mut self, other: &mut Self) {
        self.vec.append(&mut other.vec)
    }

    /// Splits the collection into two at the given index. See
    /// [`Vec::split_off`].
    #[inline]
    pub fn split_off(&mut self, idx: I) -> Self {
        Self::from_vec(self.vec.split_off(idx.index()))
    }

    /// Remove the item at `index`. See [`Vec::remove`].
    #[inline]
    pub fn remove(&mut self, index: I) -> T {
        self.vec.remove(index.index())
    }

    /// Insert an item at `index`. See [`Vec::insert`].
    #[inline]
    pub fn insert(&mut self, index: I, element: T) {
        self.vec.insert(index.index(), element)
    }

    /// Remove the item at `index` without maintaining order. See [`Vec::swap_remove`].
    #[inline]
    pub fn swap_remove(&mut self, index: I) -> T {
        self.vec.swap_remove(index.index())
    }

    /// Call `slice::binary_search` converting the indices it gives us back as
    /// needed.
    #[inline]
    pub fn binary_search(&self, value: &T) -> Result<I, I> where T: Ord {
        match self.vec.binary_search(value) {
            Ok(i) => Ok(Idx::from_usize(i)),
            Err(i) => Err(Idx::from_usize(i)),
        }
    }

    /// Append all items in the slice to the end of our vector.
    ///
    /// See [`Vec::extend_from_slice`].
    #[inline]
    pub fn extend_from_slice(&mut self, other: &[T]) where T: Clone {
        self.vec.extend_from_slice(other)
    }
}

impl<I: Idx, T> Index<I> for IndexVec<I, T> {
    type Output = T;

    #[inline]
    fn index(&self, index: I) -> &T {
        &self.vec[index.index()]
    }
}

impl<I: Idx, T> IndexMut<I> for IndexVec<I, T> {
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut T {
        &mut self.vec[index.index()]
    }
}

impl<I: Idx, T> Default for IndexVec<I, T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<I: Idx, T> Extend<T> for IndexVec<I, T> {
    #[inline]
    fn extend<J: IntoIterator<Item = T>>(&mut self, iter: J) {
        self.vec.extend(iter);
    }
}

impl<'a, I: Idx, T: 'a + Copy> Extend<&'a T> for IndexVec<I, T> {
    #[inline]
    fn extend<J: IntoIterator<Item = &'a T>>(&mut self, iter: J) {
        self.vec.extend(iter);
    }
}

impl<I: Idx, T> FromIterator<T> for IndexVec<I, T> {
    #[inline]
    fn from_iter<J>(iter: J) -> Self
    where
        J: IntoIterator<Item = T>,
    {
        IndexVec {
            vec: FromIterator::from_iter(iter),
            _marker: PhantomData,
        }
    }
}

impl<I: Idx, T> IntoIterator for IndexVec<I, T> {
    type Item = T;
    type IntoIter = vec::IntoIter<T>;

    #[inline]
    fn into_iter(self) -> vec::IntoIter<T> {
        self.vec.into_iter()
    }
}

impl<'a, I: Idx, T> IntoIterator for &'a IndexVec<I, T> {
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> slice::Iter<'a, T> {
        self.vec.iter()
    }
}

impl<'a, I: Idx, T> IntoIterator for &'a mut IndexVec<I, T> {
    type Item = &'a mut T;
    type IntoIter = slice::IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> slice::IterMut<'a, T> {
        self.vec.iter_mut()
    }
}

impl<I: Idx, T> From<Vec<T>> for IndexVec<I, T> {
    #[inline]
    fn from(v: Vec<T>) -> Self {
        Self {
            vec: v,
            _marker: PhantomData,
        }
    }
}
