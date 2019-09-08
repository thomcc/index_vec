//! This module is just for documentation purposes, and is hidden behind the
//! `example_generated` feature, which is off by default.

pub mod wraps_u32 {
    define_index_type! {
        /// Example documentation for the type
        pub struct Idx32(u32);
    }
}

pub mod wraps_usize {
    define_index_type! {
        /// Example documentation for the type.
        pub struct IdxSize(usize);
        DEFAULT = IdxSize(0);
    }
}
