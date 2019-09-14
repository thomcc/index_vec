//! This crate helps with defining "newtype"-style wrappers around `usize` (or
//! other integers), and `Vec<T>` so that some additional type safety can be
//! gained at zero cost.
//!
//! ## Example / Overview
//! ```rust
//! use index_vec::{IndexVec, index_vec};
//!
//! index_vec::define_index_type! {
//!     // Define StrIdx to use only 32 bits internally (you can use usize, u16,
//!     // and even u8).
//!     pub struct StrIdx = u32;
//!
//!     // The defaults are very reasonable, but this macro can let
//!     // you customize things quite a bit:
//!
//!     // By default, creating a StrIdx would check an incoming `usize against
//!     // `u32::max_value()`, as u32 is the wrapped index type. Lets imagine that
//!     // StrIdx has to interface with an external system that uses signed ints.
//!     // We can change the checking behavior to complain on i32::max_value()
//!     // instead:
//!     MAX_INDEX = i32::max_value() as usize;
//!
//!     // We can also disable checking all-together if we are more concerned with perf
//!     // than any overflow problems, or even do so, but only for debug builds: Quite
//!     // pointless here, but an okay example
//!     DISABLE_MAX_INDEX_CHECK = cfg!(not(debug_assertions));
//!
//!     // And more too, see this macro's docs for more info.
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
//!
//! // Addition
//! assert_eq!(StrIdx::new(0) + 1, 1usize);
//!
//! // Subtraction
//! assert_eq!(StrIdx::new(1) - 1, 0usize);
//!
//! // Wrapping
//! assert_eq!(StrIdx::new(5) % strs.len(), 1usize);
//! // ...
//! ```
//! ## Background
//!
//! The goal is to help with the pattern of using a `type FooIdx = usize` to
//! access a `Vec<Foo>` with something that can statically prevent using a
//! `FooIdx` in a `Vec<Bar>`. It's most useful if you have a bunch of indices
//! referring to different sorts of vectors.
//!
//! The code was originally based on `rustc`'s `IndexVec` code, however that has
//! been almost entirely rewritten (except for the cases where it's trivial,
//! e.g. the Vec wrapper).
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
//!
//! ## FAQ
//!
//! #### Wouldn't `define_index_type` be better as a proc macro?
//!
//! Probably. It's not a proc macro because I tend to avoid them where possible
//! due to wanting to minimize compile times. If the issues around proc-macro
//! compile times are fixed, then I'll revisit this.
//!
//! I also may eventually add a proc-macro feature which is not required, but
//! avoids some of the grossness.
//!
//! #### Does `define_index_type` do too much?
//!
//! Possibly. It defines a type, implements a bunch of functions on it, and
//! quite a few traits. That said, it's intended to be a very painless journey
//! from `Vec<T>` + `usize` to `IndexVec<I, T>`. If it left it up to the
//! developer to do those things, it would be too annoying to be worth using.
//!
//! #### The syntax for the options in `define_index_type` is terrible.
//!
//! I'm open to suggestions.
//!
//! #### What features are planned?
//!
//! Planned is a bit strong but here are the things I would find useful.
//!
//! - Extend the model to include a slice type, which should improve ergonomics
//!   in some places.
//! - Support any remaining parts of the slice/vec api.
//! - Add typesafe wrappers for SmallVec/ArrayVec (behind a cargo `feature`, of
//!   course).
//! - Better syntax for the define_index_type macro (no concrete ideas).
//! - Allow the generated type to be a tuple struct, or use a specific field
//!   name.
//! - Allow use of indices for string types (the primary benefit here would
//!   probably be the ability to e.g. use u32 without too much pain rather than
//!   mixing up indices from different strings -- but you never know!)
//! - Allow index types such as NonZeroU32 and such, if it can be done sanely.
//! - ...
//!
#![allow(clippy::partialeq_ne_impl)]
#![no_std]
extern crate alloc;

use alloc::borrow::{Cow, ToOwned};
use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::borrow::{Borrow, BorrowMut};
use core::fmt;
use core::fmt::Debug;
use core::hash::Hash;
use core::iter::{self, FromIterator};
use core::marker::PhantomData;
use core::ops::Range;
use core::slice;
mod idxslice;
mod indexing;
pub use idxslice::*;
pub use indexing::{IdxRangeBounds, IdxSliceIndex};

#[macro_use]
mod macros;
pub use macros::*;

#[cfg(any(test, feature = "example_generated"))]
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
/// generally panicing, unless it was turned off via the
/// `DISABLE_MAX_INDEX_CHECK` option in [`define_index_type!`]. If you need more
/// subtle handling than this, then you're on your own (or, well, either handle
/// it earlier, or pick a bigger index type).
///
/// Note: I'm open for suggestions on how to handle this case, but do not want
/// the typical cases (E.g. Idx is a newtyped usize or u32), to become more
/// complex.
pub trait Idx: Copy + 'static + Ord + Debug + Hash {
    /// Construct an Index from a usize. This is equivalent to From<usize>.
    ///
    /// Note that this will panic if `idx` does not fit (unless checking has
    /// been disabled, as mentioned above). Also note that `Idx` implementations
    /// are free to define what "fit" means as they desire.
    fn from_usize(idx: usize) -> Self;

    /// Get the underlying index. This is equivalent to Into<usize>
    fn index(self) -> usize;
}

/// A macro equivalent to the stdlib's `vec![]`, but producing an `IndexVec`.
#[macro_export]
macro_rules! index_vec {
    ($($tokens:tt)*) => {
        $crate::IndexVec::from_vec(vec![$($tokens)*])
    }
}

/// A Vec that only accepts indices of a specific type.
///
/// This is a thin wrapper around `Vec`, to the point where the backing vec is a
/// public property. This is in part because I know this API is not a complete
/// mirror of Vec's (patches welcome). In the worst case, you can always do what
/// you need to the Vec itself.
///
/// ## Some notes on the APIs
///
/// - Most of the Vec/Slice APIs are present.
///     - Any that aren't can be trivially accessed on the underlying `vec`
///       field, which is public.
///     - Most of the ones that aren't re-exposed I'm still thinking about, I
///       belive I got the obvious ones.
///
/// - Apis that take or return usizes referring to the positions of items were
///   replaced with ones that take Idx.
///
/// - Apis that take `R: RangeBounds<usize>` take an
///   [`IdxRangeBounds<I>`][IdxRangeBounds], which is basically a
///   `RangeBounds<I>`.
///
/// - Most iterator functions where `the_iter().enumerate()` would refer to
///   indices have been given `_enumerated` variants. E.g.
///   [`IndexVec::iter_enumerated`], [`IndexVec::drain_enumerated`], etc. This
///   is because `v.iter().enumerate()` would be `(usize, &T)`, but you want
///   `(I, &T)`.
///
/// ## APIs not present on `Vec<T>` or `[T]`
///
/// The following extensions are added:
///
/// - [`IndexVec::indices`]: an Iterator over the indices of type `I`.
/// - [`IndexVec::next_idx`], [`IndexVec::last_idx`] give the next and most
///   recent index returned by `push`.
/// - [`IndexVec::push`] returns the index the item was inserted at.
/// - Various `enumerated` iterators mentioned earlier
/// - [`IndexVec::position`], [`IndexVec::rposition`] as
///   `self.iter().position()` will return a `Option<usize>`
///
/// ## Pitfalls / Gotchas
///
/// - `IndexVec<I, T>` is not `Deref<Target = [T]>`. This means it does not
///   auto-coerce into &[T] when needed. I think this is for the best, but could
///   probably be convinced otherwise.
///
///   At the moment, we attempt to make up for this by wrapping the bulk of the
///   API for slices as well, but still. Note that you still can access the vec
///   directly whenever you need.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
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

    /// Similar to `self.into_iter().enumerate()` but with indices of `I` and
    /// not `usize`.
    #[inline]
    pub fn into_iter_enumerated(self) -> Enumerated<vec::IntoIter<T>, I, T> {
        self.vec
            .into_iter()
            .enumerate()
            .map(|(i, t)| (Idx::from_usize(i), t))
    }

    // /// Similar to `self.iter().enumerate()` but with indices of `I` and not
    // /// `usize`.
    // #[inline]
    // pub fn iter_enumerated(&self) -> Enumerated<slice::Iter<'_, T>, I, &T> {
    //     self.vec
    //         .iter()
    //         .enumerate()
    //         .map(|(i, t)| (Idx::from_usize(i), t))
    // }

    // /// Get an interator over all our indices.
    // #[inline]
    // pub fn indices(&self) -> iter::Map<Range<usize>, fn(usize) -> I> {
    //     (0..self.len()).map(Idx::from_usize)
    // }

    /// Creates a splicing iterator that replaces the specified range in the
    /// vector with the given `replace_with` iterator and yields the removed
    /// items. See [`Vec::splice`]
    pub fn splice<R, It>(
        &mut self,
        range: R,
        replace_with: It,
    ) -> vec::Splice<<It as IntoIterator>::IntoIter>
    where
        It: IntoIterator<Item = T>,
        R: IdxRangeBounds<I>,
    {
        self.vec.splice(range.into_range(), replace_with)
    }

    // /// Similar to `self.iter_mut().enumerate()` but with indices of `I` and not
    // /// `usize`.
    // #[inline]
    // pub fn iter_mut_enumerated(&mut self) -> Enumerated<slice::IterMut<'_, T>, I, &mut T> {
    //     self.vec
    //         .iter_mut()
    //         .enumerate()
    //         .map(|(i, t)| (Idx::from_usize(i), t))
    // }

    /// Similar to `self.drain(r).enumerate()` but with indices of `I` and not
    /// `usize`.
    #[inline]
    pub fn drain_enumerated<R: IdxRangeBounds<I>>(
        &mut self,
        range: R,
    ) -> Enumerated<vec::Drain<'_, T>, I, T> {
        self.vec
            .drain(range.into_range())
            .enumerate()
            .map(|(i, t)| (Idx::from_usize(i), t))
    }

    /// Gives the next index that will be assigned when `push` is
    /// called.
    #[inline]
    pub fn next_idx(&self) -> I {
        I::from_usize(self.len())
    }

    // /// Return the index of the last element, or panic.
    // #[inline]
    // pub fn last_idx(&self) -> I {
    //     // TODO: should this still be a panic even when `I` has disabled
    //     // checking?
    //     assert!(!self.is_empty());
    //     I::from_usize(self.len() - 1)
    // }

    /// Get a the storage as a `&[T]`
    #[inline]
    pub fn as_raw_slice(&self) -> &[T] {
        &self.vec
    }

    /// Get a the storage as a `&mut [T]`
    #[inline]
    pub fn as_raw_slice_mut(&mut self) -> &mut [T] {
        &mut self.vec
    }

    /// Equivalent to accessing our `vec` field, but as a function.
    #[inline]
    pub fn as_vec(&self) -> &Vec<T> {
        &self.vec
    }

    /// Equivalent to accessing our `vec` field mutably, but as a function, if
    /// that's what you'd prefer.
    #[inline]
    pub fn as_mut_vec(&mut self) -> &mut Vec<T> {
        &mut self.vec
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

    /// Converts the vector into an owned IdxSlice, dropping excess capacity.
    pub fn into_boxed_slice(self) -> alloc::boxed::Box<IdxSlice<I, [T]>> {
        let b = self.vec.into_boxed_slice();
        unsafe { Box::from_raw(Box::into_raw(b) as *mut IdxSlice<I, [T]>) }
    }

    // /// Returns the length of our vector.
    // #[inline]
    // pub fn len(&self) -> usize {
    //     self.vec.len()
    // }

    // /// Returns true if we're empty.
    // #[inline]
    // pub fn is_empty(&self) -> bool {
    //     self.vec.is_empty()
    // }

    // /// Get a iterator over reverences to our values.
    // ///
    // /// See also [`IndexVec::iter_enumerated`], which gives you indices (of the
    // /// correct type) as you iterate.
    // #[inline]
    // pub fn iter(&self) -> slice::Iter<'_, T> {
    //     self.vec.iter()
    // }

    // /// Get a iterator over mut reverences to our values.
    // ///
    // /// See also [`IndexVec::iter_mut_enumerated`], which gives you indices (of
    // /// the correct type) as you iterate.
    // #[inline]
    // pub fn iter_mut(&mut self) -> slice::IterMut<'_, T> {
    //     self.vec.iter_mut()
    // }

    /// Return an iterator that removes the items from the requested range. See
    /// [`Vec::drain`].
    ///
    /// See also [`IndexVec::drain_enumerated`], which gives you indices (of the
    /// correct type) as you iterate.
    #[inline]
    pub fn drain<R: IdxRangeBounds<I>>(&mut self, range: R) -> vec::Drain<'_, T> {
        self.vec.drain(range.into_range())
    }

    // /// Return the index of the last element, if we are not empty.
    // #[inline]
    // pub fn last(&self) -> Option<I> {
    //     self.len().checked_sub(1).map(I::from_usize)
    // }

    /// Shrinks the capacity of the vector as much as possible.
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.vec.shrink_to_fit()
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

    /// Get a ref to the item at the provided index, or None for out of bounds.
    #[inline]
    pub fn get<J: IdxSliceIndex<I, T>>(&self, index: J) -> Option<&J::Output> {
        index.get(self.as_slice())
    }

    /// Get a mut ref to the item at the provided index, or None for out of
    /// bounds
    #[inline]
    pub fn get_mut<J: IdxSliceIndex<I, T>>(&mut self, index: J) -> Option<&mut J::Output> {
        index.get_mut(self.as_mut_slice())
    }

    /// Returns a reference to an element, without doing bounds checking.
    ///
    /// This is generally not recommended, use with caution!
    #[inline]
    pub unsafe fn get_unchecked<J: IdxSliceIndex<I, T>>(&self, index: J) -> &J::Output {
        index.get_unchecked(self.as_slice())
    }

    /// Returns a mutable reference to an element or subslice, without doing
    /// bounds checking.
    ///
    /// This is generally not recommended, use with caution!
    #[inline]
    pub unsafe fn get_unchecked_mut<J: IdxSliceIndex<I, T>>(&mut self, index: J) -> &mut J::Output {
        index.get_unchecked_mut(self.as_mut_slice())
    }

    /// Resize ourselves in-place to `new_len`. See [`Vec::resize`].
    #[inline]
    pub fn resize(&mut self, new_len: usize, value: T)
    where
        T: Clone,
    {
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

    /// Remove the item at `index` without maintaining order. See
    /// [`Vec::swap_remove`].
    #[inline]
    pub fn swap_remove(&mut self, index: I) -> T {
        self.vec.swap_remove(index.index())
    }

    /// Insert an item at `index`. See [`Vec::insert`].
    #[inline]
    pub fn insert(&mut self, index: I, element: T) {
        self.vec.insert(index.index(), element)
    }

    // /// Searches for an element in an iterator, returning its index. This is
    // /// equivalent to `Iterator::position`, but returns `I` and not `usize`.
    // #[inline]
    // pub fn position<F: FnMut(&T) -> bool>(&self, f: F) -> Option<I> {
    //     self.vec.iter().position(f).map(Idx::from_usize)
    // }

    // /// Searches for an element in an iterator from the right, returning its
    // /// index. This is equivalent to `Iterator::position`, but returns `I` and
    // /// not `usize`.
    // #[inline]
    // pub fn rposition<F: FnMut(&T) -> bool>(&self, f: F) -> Option<I> {
    //     self.vec.iter().rposition(f).map(Idx::from_usize)
    // }

    // /// Swaps two elements in our vector.
    // #[inline]
    // pub fn swap(&mut self, a: I, b: I) {
    //     self.vec.swap(a.index(), b.index())
    // }

    // /// Divides our slice into two at an index.
    // #[inline]
    // pub fn split_at(&self, a: I) -> (&[T], &[T]) {
    //     self.vec.split_at(a.index())
    // }

    // /// Divides our slice into two at an index.
    // #[inline]
    // pub fn split_at_mut(&mut self, a: I) -> (&mut [T], &mut [T]) {
    //     self.vec.split_at_mut(a.index())
    // }

    // /// Rotates our data in-place such that the first `mid` elements of the
    // /// slice move to the end while the last `self.len() - mid` elements move to
    // /// the front
    // #[inline]
    // pub fn rotate_left(&mut self, mid: I) {
    //     self.vec.rotate_left(mid.index())
    // }

    // /// Rotates our data in-place such that the first `self.len() - k` elements
    // /// of the slice move to the end while the last `k` elements move to the
    // /// front
    // #[inline]
    // pub fn rotate_right(&mut self, k: I) {
    //     self.vec.rotate_right(k.index())
    // }

    // /// Copies elements from one part of the slice to another part of itself,
    // /// using a memmove.
    // #[inline]
    // pub fn copy_within<R: IdxRangeBounds<I>>(&mut self, src: R, dst: I)
    // where
    //     T: Copy,
    // {
    //     self.vec.copy_within(src.into_range(), dst.index())
    // }

    // /// Call `slice::binary_search` converting the indices it gives us back as
    // /// needed.
    // #[inline]
    // pub fn binary_search(&self, value: &T) -> Result<I, I>
    // where
    //     T: Ord,
    // {
    //     match self.vec.binary_search(value) {
    //         Ok(i) => Ok(Idx::from_usize(i)),
    //         Err(i) => Err(Idx::from_usize(i)),
    //     }
    // }

    // /// Binary searches this sorted vec with a comparator function, converting
    // /// the indices it gives us back to our Idx type.
    // #[inline]
    // pub fn binary_search_by<'a, F: FnMut(&'a T) -> core::cmp::Ordering>(
    //     &'a self,
    //     f: F,
    // ) -> Result<I, I> {
    //     match self.vec.binary_search_by(f) {
    //         Ok(i) => Ok(Idx::from_usize(i)),
    //         Err(i) => Err(Idx::from_usize(i)),
    //     }
    // }

    // /// Binary searches this sorted vec with a key extraction function, converting
    // /// the indices it gives us back to our Idx type.
    // #[inline]
    // pub fn binary_search_by_key<'a, B: Ord, F: FnMut(&'a T) -> B>(
    //     &'a self,
    //     b: &B,
    //     f: F,
    // ) -> Result<I, I> {
    //     match self.vec.binary_search_by_key(b, f) {
    //         Ok(i) => Ok(Idx::from_usize(i)),
    //         Err(i) => Err(Idx::from_usize(i)),
    //     }
    // }

    /// Append all items in the slice to the end of our vector.
    ///
    /// See [`Vec::extend_from_slice`].
    #[inline]
    pub fn extend_from_slice(&mut self, other: &IdxSlice<I, [T]>)
    where
        T: Clone,
    {
        self.vec.extend_from_slice(&other.slice)
    }

    // /// Forwards to the `Vec::sort_unstable` implementation.
    // #[inline]
    // pub fn sort_unstable(&mut self)
    // where
    //     T: Ord,
    // {
    //     self.vec.sort_unstable()
    // }

    // /// Forwards to the `Vec::sort_unstable_by` implementation.
    // #[inline]
    // pub fn sort_unstable_by<F: FnMut(&T, &T) -> core::cmp::Ordering>(&mut self, compare: F) {
    //     self.vec.sort_unstable_by(compare)
    // }

    // /// Forwards to the `Vec::sort_unstable_by_key` implementation.
    // #[inline]
    // pub fn sort_unstable_by_key<F: FnMut(&T) -> K, K: Ord>(&mut self, f: F) {
    //     self.vec.sort_unstable_by_key(f)
    // }

    // /// Forwards to the `Vec::ends_with` implementation.
    // #[inline]
    // pub fn ends_with(&self, needle: &[T]) -> bool
    // where
    //     T: PartialEq,
    // {
    //     self.vec.ends_with(needle)
    // }

    // /// Forwards to the `Vec::starts_with` implementation.
    // #[inline]
    // pub fn starts_with(&self, needle: &[T]) -> bool
    // where
    //     T: PartialEq,
    // {
    //     self.vec.starts_with(needle)
    // }

    // /// Forwards to the `Vec::contains` implementation.
    // #[inline]
    // pub fn contains(&self, x: &T) -> bool
    // where
    //     T: PartialEq,
    // {
    //     self.vec.contains(x)
    // }

    /// Forwards to the `Vec::retain` implementation.
    #[inline]
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, f: F) {
        self.vec.retain(f)
    }

    /// Forwards to the `Vec::dedup_by_key` implementation.
    #[inline]
    pub fn dedup_by_key<F: FnMut(&mut T) -> K, K: PartialEq>(&mut self, key: F) {
        self.vec.dedup_by_key(key)
    }

    /// Forwards to the `Vec::dedup` implementation.
    #[inline]
    pub fn dedup(&mut self)
    where
        T: PartialEq,
    {
        self.vec.dedup()
    }

    /// Forwards to the `Vec::dedup_by` implementation.
    #[inline]
    pub fn dedup_by<F: FnMut(&mut T, &mut T) -> bool>(&mut self, same_bucket: F) {
        self.vec.dedup_by(same_bucket)
    }

    /// Forwards to the `Vec::reverse` implementation.
    #[inline]
    pub fn reverse(&mut self) {
        self.vec.reverse()
    }

    /// Get a IdxSlice over this vector.
    #[inline]
    pub fn as_slice(&self) -> &IdxSlice<I, [T]> {
        IdxSlice::new(&self.vec)
    }

    /// Get a mutable IdxSlice over this vector.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut IdxSlice<I, [T]> {
        IdxSlice::new_mut(&mut self.vec)
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

impl<I: Idx, T> From<IndexVec<I, T>> for Box<IdxSlice<I, [T]>> {
    #[inline]
    fn from(src: IndexVec<I, T>) -> Self {
        src.into_boxed_slice()
    }
}

impl<I: Idx, T> From<Box<IdxSlice<I, [T]>>> for IndexVec<I, T> {
    #[inline]
    fn from(src: Box<IdxSlice<I, [T]>>) -> Self {
        src.into_vec()
    }
}

impl<'a, I: Idx, T> From<Cow<'a, IdxSlice<I, [T]>>> for IndexVec<I, T>
where
    IdxSlice<I, [T]>: ToOwned<Owned = IndexVec<I, T>>,
{
    fn from(s: Cow<'a, IdxSlice<I, [T]>>) -> IndexVec<I, T> {
        s.into_owned()
    }
}

impl<'a, I: Idx, T: Clone> From<&'a IdxSlice<I, [T]>> for IndexVec<I, T> {
    #[inline]
    fn from(src: &'a IdxSlice<I, [T]>) -> Self {
        src.to_owned()
    }
}
impl<'a, I: Idx, T: Clone> From<&'a mut IdxSlice<I, [T]>> for IndexVec<I, T> {
    #[inline]
    fn from(src: &'a mut IdxSlice<I, [T]>) -> Self {
        src.to_owned()
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

impl<I: Idx, T: Clone> Clone for IndexVec<I, T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            vec: self.vec.clone(),
            _marker: PhantomData,
        }
    }
    #[inline]
    fn clone_from(&mut self, o: &Self) {
        self.vec.clone_from(&o.vec);
    }
}

impl<I: Idx, A> AsRef<[A]> for IndexVec<I, A> {
    #[inline]
    fn as_ref(&self) -> &[A] {
        &self.vec
    }
}

impl<I: Idx, A> AsMut<[A]> for IndexVec<I, A> {
    #[inline]
    fn as_mut(&mut self) -> &mut [A] {
        &mut self.vec
    }
}

impl<I: Idx, A> AsRef<IdxSlice<I, [A]>> for IndexVec<I, A> {
    #[inline]
    fn as_ref(&self) -> &IdxSlice<I, [A]> {
        IdxSlice::new(&self.vec)
    }
}

impl<I: Idx, A> AsMut<IdxSlice<I, [A]>> for IndexVec<I, A> {
    #[inline]
    fn as_mut(&mut self) -> &mut IdxSlice<I, [A]> {
        IdxSlice::new_mut(&mut self.vec)
    }
}

impl<I: Idx, A> core::ops::Deref for IndexVec<I, A> {
    type Target = IdxSlice<I, [A]>;
    #[inline]
    fn deref(&self) -> &IdxSlice<I, [A]> {
        IdxSlice::new(&self.vec)
    }
}

impl<I: Idx, A> core::ops::DerefMut for IndexVec<I, A> {
    #[inline]
    fn deref_mut(&mut self) -> &mut IdxSlice<I, [A]> {
        IdxSlice::new_mut(&mut self.vec)
    }
}

impl<I: Idx, T> Borrow<IdxSlice<I, [T]>> for IndexVec<I, T> {
    #[inline]
    fn borrow(&self) -> &IdxSlice<I, [T]> {
        self.as_slice()
    }
}

impl<I: Idx, T> BorrowMut<IdxSlice<I, [T]>> for IndexVec<I, T> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut IdxSlice<I, [T]> {
        self.as_mut_slice()
    }
}

macro_rules! impl_partialeq {
    ($Lhs: ty, $Rhs: ty) => {
        impl<'a, 'b, A, B, I: Idx> PartialEq<$Rhs> for $Lhs
        where
            A: PartialEq<B>,
        {
            #[inline]
            fn eq(&self, other: &$Rhs) -> bool {
                self[..] == other[..]
            }
            #[inline]
            fn ne(&self, other: &$Rhs) -> bool {
                self[..] != other[..]
            }
        }
    };
}

impl_partialeq! { IndexVec<I, A>, Vec<B> }
impl_partialeq! { IndexVec<I, A>, &'b [B] }
impl_partialeq! { IndexVec<I, A>, &'b mut [B] }

impl_partialeq! { IndexVec<I, A>, &'b IdxSlice<I, [B]> }
impl_partialeq! { IndexVec<I, A>, &'b mut IdxSlice<I, [B]> }

impl_partialeq! { &'a IdxSlice<I, [A]>, Vec<B> }
impl_partialeq! { &'a mut IdxSlice<I, [A]>, Vec<B> }

impl_partialeq! { &'a IdxSlice<I, [A]>, IndexVec<I, B> }
impl_partialeq! { &'a mut IdxSlice<I, [A]>, IndexVec<I, B> }
// impl_partialeq! { &'a IdxSlice<I, [A]>, &'b [B] }
// impl_partialeq! { &'a IdxSlice<I, [A]>, &'b mut [B] }

macro_rules! array_impls {
    ($($N: expr)+) => {$(
        impl_partialeq! { IndexVec<I, A>, [B; $N] }
        impl_partialeq! { IndexVec<I, A>, &'b [B; $N] }
        impl_partialeq! { &'a IdxSlice<I, [A]>, [B; $N] }
        impl_partialeq! { &'a IdxSlice<I, [A]>, &'b [B; $N] }
    )+};
}

array_impls! {
     0  1  2  3  4  5  6  7  8  9
    10 11 12 13 14 15 16 17 18 19
    20 21 22 23 24 25 26 27 28 29
    30 31 32
}

#[inline(never)]
#[cold]
#[doc(hidden)]
pub fn __max_check_fail(u: usize, max: usize) -> ! {
    panic!(
        "index_vec index overflow: {} is outside the range [0, {})",
        u, max,
    )
}

#[cfg(feature = "serde")]
impl<I: Idx, T: serde::ser::Serialize> serde::ser::Serialize for IndexVec<I, T> {
    fn serialize<S: serde::ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.vec.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, I: Idx, T: serde::de::Deserialize<'de>> serde::de::Deserialize<'de> for IndexVec<I, T> {
    fn deserialize<D: serde::de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Vec::deserialize(deserializer).map(Self::from_vec)
    }
}
