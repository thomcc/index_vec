# `index_vec`

A library mainly providing:

- `IndexVec<IndexType, Value>`, a thin wrapper around `Vec` which only requires
  you use `IndexType` to index into it.

- A macro for defining the boilerplate required to make using newtyped indices
  palatable.

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

## Example

```rust
use index_vec::{IndexVec, index_vec};

index_vec::define_index_type! {
    pub struct MyIdx(u32);
}

let foo: IndexVec<MyIdx, &'static str> = index_vec!["foo", "bar", "baz"];
// Now, only `MyIdx`s can index into `foo`.
```

## License

This is based on code from `rustc`'s source, and retains it's status as
dual-licensed under MIT (LICENSE-MIT) / Apache 2.0 (LICENSE-APACHE).
