# `index_vec`

[![Docs](https://docs.rs/index_vec/badge.svg)](https://docs.rs/index_vec) ![crates.io](https://img.shields.io/crates/v/index_vec.svg) [![CircleCI](https://circleci.com/gh/thomcc/index_vec.svg?style=svg)](https://circleci.com/gh/thomcc/index_vec)

**Note: API still subject to change for a bit prior to 0.1.0**

A more type-safe version of using `Vec`, for when `usize`s are getting you down.

This library mainly provides `IndexVec<I, T>`, which wraps `Vec` so that it's
accessed with `I` and not `usize`.

It also defines a macro for defining new index types (for use with IndexVec).
Mostly outputting boilerplate.

## Example / Overview

```rust
use index_vec::{IndexVec, index_vec};

// Define a custom index type.
index_vec::define_index_type! {
    // Define StrIdx to use only 32 bits internally (you can use usize, u16,
    // and even u8).
    pub struct StrIdx = u32;
    // Note that this macro has a decent amount of configurability, so
    // be sure to read its documentation if you think it's doing
    // something you don't want.
}

// Create a vector which can be accessed using `StrIdx`s.
let mut strs: IndexVec<StrIdx, &'static str> = index_vec!["strs", "bar", "baz"];

// l is a `StrIdx`
let l = strs.last_idx();
assert_eq!(strs[l], "baz");

let new_i = strs.push("quux");
assert_eq!(strs[new_i], "quux");

// Indices are mostly interoperable with `usize`, and support
// a lot of what you might want to do to an index. (Note that
// it does *not* support these with other index wrappers --
// that seems too likely to lead to bugs).

// Comparison
assert_eq!(StrIdx::new(0), 0usize);
// Addition
assert_eq!(StrIdx::new(0) + 1, 1usize);

// Subtraction (Note that by default, the index will panic on overflow,
// but that can be configured in the macro)
assert_eq!(StrIdx::new(1) - 1, 0usize);

// Wrapping
assert_eq!(StrIdx::new(5) % strs.len(), 1usize);
// ...
```
## Background

The goal is to replace the pattern of using a `type FooIdx = usize` to access a
`Vec<Foo>` with something that can statically prevent using a `FooIdx` in a
`Vec<Bar>`. It's most useful if you have a bunch of indices referring to
different sorts of vectors.

Much of the code for this is taken from `rustc`'s `IndexVec` code, however it's
diverged a decent amount at this point. The largest differences are:

- No usage of unstable features.
- Different syntax for defining index types.
- Allows use of index types beyond `u32` (`usize`, `u32`, `u16`, and `u8` are
  all supported).
- More flexible behavior around how strictly some checks are performed.

## Other crates

The [`indexed_vec`](https://crates.io/crates/indexed_vec) crate predates this,
and is a much closer copy of the code from `rustc`. Unfortunately, this means it
does not compile on stable.

If you're looking for something further from a vec and closer to a map, you might find [`handy`](https://crates.io/crates/handy), [`slotmap`](https://crates.io/crates/slotmap), or [`slab`](https://crates.io/crates/slab) to be closer what you want.

## License

This is based on code from `rustc`'s source, and retains it's status as
dual-licensed under MIT (LICENSE-MIT) / Apache 2.0 (LICENSE-APACHE).
