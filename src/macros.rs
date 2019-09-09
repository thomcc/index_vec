/// Generate the boilerplate for a newtyped index struct, for use with
/// `IndexVec`.
///
/// ## Usage
///
/// ### Standard
///
/// The rough usage pattern of this macro is:
///
/// ```rust,no_run
/// index_vec::define_index_type! {
///     // Note that isn't actually a type alias, `MyIndex` is
///     // actually defined as a struct. XXX is this too confusing?
///     pub struct MyIndex = u32;
///     // optional extra configuration here of the form:
///     // `OPTION_NAME = stuff;`
///     // See below for details.
/// }
/// ```
///
/// Note that you can use other index types than `u32`, and you can set it to be
/// `MyIndex(pub u32)` as well. Currently, the wrapped item be a tuple struct,
/// however (patches welcome).
///
/// ### Customization
///
/// After the struct declaration, there are a number of configuration options
/// the macro uses to customize how the type it generates behaves. For example:
///
/// ```rust,no_run
/// index_vec::define_index_type! {
///     #[repr(transparent)]
///     pub struct Span = u32;
///
///     // Don't allow any spans with values higher this.
///     MAX_INDEX = 0x7fff_ff00;
///
///     // But I also am not too worried about it, so only
///     // perform the asserts in debug builds.
///     DISABLE_MAX_INDEX_CHECK = !cfg!(debug_assertions);
/// }
/// ```
///
/// ## Configuration options
///
/// This macro has a few ways you can customize it's output behavior. There's
/// not really any great syntax I can think of for them, but, well.
///
/// #### `MAX_INDEX = <expr producing usize>`
///
/// Assert if anything tries to construct an index above that value.
///
/// By default, this is `$raw_type::max_value() as usize`, e.g. we check that
/// our cast from `usize` to our wrapper is lossless, but we assume any all
/// instance of `$raw_type` is valid in this index domain.
///
/// Note that these tests can be disabled entirely, or conditionally, with
/// `DISABLE_MAX_INDEX_CHECK`. Additionally, the generated type has
/// `from_usize_unchecked` and `from_raw_unchecked` functions which can be used
/// to ignore these checks.
///
/// #### `DISABLE_MAX_INDEX_CHECK = <expr>;`
///
/// Set to true to disable the assertions mentioned above. False by default.
///
/// To be clear, if this is set to false, we blindly assume all casts between
/// `usize` and `$raw_type` succeed.
///
/// A common use is setting `DISABLE_MAX_INDEX_CHECK = !cfg!(debug_assertions)` to
/// avoid the tests at compile time
///
/// For the sake of clarity, disabling this cannot lead to memory unsafety -- we
/// still go through bounds checks when accessing slices, and no unsafe code
/// (unless you write some, and don't! only use this for correctness!) should
/// rely on on these checks.
///
/// #### `DEFAULT = <expr>;`
/// If provided, we'll implement `Default` for the index type using this
/// expresson.
///
/// Example:
///
/// ```rust,no_run
/// index_vec::define_index_type! {
///     pub struct MyIdx = u16;
///     MAX_INDEX = (u16::max_value() - 1) as usize;
///     // Set the default index to be an invalid index, as
///     // a hacky way of having this type behave somewhat
///     // like it were an Option<MyIdx> without consuming
///     // extra space.
///     DEFAULT = (MyIdx::from_raw_unchecked(u16::max_value()));
/// }
/// ```
///
/// #### `NO_DERIVES = true;`
///
/// By default the generated type will `derive` all traits needed to make itself
/// work. Specifically, `Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd,
/// Ord`. If you'd like to provide your own implementation of one of these, this
/// is a problem.
///
/// It can be worked around by setting NO_DERIVES, and providing the
/// implementations yourself, usually with a combination of implementing it
/// manually and using Derives, for example, if I want to use a custom `Debug`
/// impl:
///
/// ```rust,no_run
/// index_vec::define_index_type! {
///     // Derive everything needs except `Debug`.
///     #[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
///     struct MyIdx = usize;
///     NO_DERIVES = true;
/// }
/// // and then implement Debug manually.
/// impl core::fmt::Debug for MyIdx {
///    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
///        write!(f, "{}", self.raw())
///    }
/// }
/// ```
#[macro_export]
macro_rules! define_index_type {
    // public api
    (
        $(#[$attrs:meta])*
        $v:vis struct $type:ident = $raw:ident;
        $($config:tt)*
    ) => {
        $crate::define_index_type!{
            @__inner
            @attrs [$(#[$attrs])*]
            @derives [#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]]
            @decl [$v struct $type ($raw)]
            @max [(<$raw>::max_value() as usize)]
            @no_check_max [false]
            { $($config)* }
        }
    };

    // DISABLE_MAX_INDEX_CHECK
    (@__inner
        @attrs [$(#[$attrs:meta])*]
        @derives [$(#[$derive:meta])*]
        @decl [$v:vis struct $type:ident ($raw:ident)]
        @max [$max:expr]
        @no_check_max [$_old_no_check_max:expr]
        { DISABLE_MAX_INDEX_CHECK = $no_check_max:expr; $($tok:tt)* }
    ) => {
        $crate::define_index_type!{
            @__inner
            @attrs [$(#[$attrs])*]
            @derives [$(#[$derive])*]
            @decl [$v struct $type ($raw)]
            @max [$max]
            @no_check_max [$no_check_max]
            { $($tok)* }
        }
    };

    // MAX_INDEX
    (@__inner
        @attrs [$(#[$attrs:meta])*]
        @derives [$(#[$derive:meta])*]
        @decl [$v:vis struct $type:ident ($raw:ident)]
        @max [$max:expr]
        @no_check_max [$cm:expr]
        { MAX_INDEX = $new_max:expr; $($tok:tt)* }
    ) => {
        $crate::define_index_type!{
            @__inner
            @attrs [$(#[$attrs])*]
            @derives [$(#[$derive])*]
            @decl [$v struct $type ($raw)]
            @max [$new_max]
            @no_check_max [$cm]
            { $($tok)* }
        }
    };

    // DEFAULT
    (@__inner
        @attrs [$(#[$attrs:meta])*]
        @derives [$(#[$derive:meta])*]
        @decl [$v:vis struct $type:ident ($raw:ident)]
        @max [$max:expr]
        @no_check_max [$no_check_max:expr]
        { DEFAULT = $default_expr:expr; $($tok:tt)* }
    ) => {
        $crate::define_index_type!{
            @__inner
            @attrs [$(#[$attrs])*]
            @derives [$(#[$derive])*]
            @decl [$v struct $type ($raw)]
            @max [$max]
            @no_check_max [$no_check_max]
            { $($tok)* }
        }
        impl Default for $type {
            #[inline]
            fn default() -> Self {
                $default_expr
            }
        }
    };

    // NO_DERIVES
    (@__inner
        @attrs [$(#[$attrs:meta])*]
        @derives [$(#[$derive:meta])*]
        @decl [$v:vis struct $type:ident ($raw:ident)]
        @max [$max:expr]
        @no_check_max [$no_check_max:expr]
        { NO_DERIVES = true; $($tok:tt)* }
    ) => {
        $crate::define_index_type!{
            @__inner
            @attrs [$(#[$attrs])*]
            @derives []
            @decl [$v struct $type ($raw)]
            @max [$max]
            @no_check_max [$no_check_max]
            { $($tok)* }
        }
    };

    // finish
    (@__inner
        @attrs [$(#[$attrs:meta])*]
        @derives [$(#[$derive:meta])*]
        @decl [$v:vis struct $type:ident ($raw:ident)]
        @max [$max:expr]
        @no_check_max [$no_check_max:expr]
        { }
    ) => {

        $(#[$derive])*
        $(#[$attrs])*
        $v struct $type { _raw: $raw }
        #[allow(clippy::cast_lossless, clippy::unnecessary_cast)]
        impl $type {
            /// If `Self::CHECKS_MAX_INDEX` is true, we'll assert if trying to
            /// produce a value larger than this in any of the ctors that don't
            /// have `unchecked` in their name.
            $v const MAX_INDEX: usize = $max;

            /// Does this index type assert if asked to construct an index
            /// larger than MAX_INDEX?
            $v const CHECKS_MAX_INDEX: bool = !$no_check_max;

            /// Construct this index type from a usize. Alias for `from_usize`.
            #[inline]
            $v fn new(value: usize) -> Self {
                Self::from_usize(value)
            }

            /// Construct this index type from the wrapped integer type.
            #[inline]
            $v fn from_raw(value: $raw) -> Self {
                Self::from_usize(value as usize)
            }

            /// Construct this index type from one in a different domain
            #[inline]
            $v fn from_foreign<F: $crate::Idx>(value: F) -> Self {
                Self::from_usize(value.index())
            }

            /// Construct from a usize without any checks.
            #[inline]
            $v const fn from_usize_unchecked(value: usize) -> Self {
                Self { _raw: value as $raw }
            }

            /// Construct from the underlying type without any checks.
            #[inline]
            $v const fn from_raw_unchecked(raw: $raw) -> Self {
                Self { _raw: raw }
            }

            /// Construct this index type from a usize.
            #[inline]
            $v fn from_usize(value: usize) -> Self {
                Self::maybe_check_index(value as usize);
                Self { _raw: value as $raw }
            }

            /// Get the wrapped index as a usize.
            #[inline]
            $v fn index(self) -> usize {
                self._raw as usize
            }

            /// Get the wrapped index.
            #[inline]
            $v fn raw(self) -> $raw {
                self._raw
            }

            /// Asserts `v <= Self::MAX_INDEX` unless Self::CHECKS_MAX_INDEX is false.
            #[inline]
            $v fn maybe_check_index(v: usize) {
                if Self::CHECKS_MAX_INDEX && (v > Self::MAX_INDEX) {
                    Self::max_check_fail(v);
                }
            }

            #[inline(never)]
            #[cold]
            fn max_check_fail(u: usize) {
                core::panic!(
                    "index_vec index overfow: {} is outside the range [0, {})",
                    u,
                    Self::MAX_INDEX,
                );
            }

            const _ENSURE_RAW_IS_UNSIGNED: [(); 0] = [(); <$raw>::min_value() as usize];
        }

        impl core::cmp::PartialOrd<usize> for $type {
            #[inline]
            fn partial_cmp(&self, other: &usize) -> Option<core::cmp::Ordering> {
                self.index().partial_cmp(other)
            }
        }

        impl core::cmp::PartialOrd<$type> for usize {
            #[inline]
            fn partial_cmp(&self, other: &$type) -> Option<core::cmp::Ordering> {
                self.partial_cmp(&other.index())
            }
        }

        impl PartialEq<usize> for $type {
            #[inline]
            fn eq(&self, other: &usize) -> bool {
                self.index() == *other
            }
        }

        impl PartialEq<$type> for usize {
            #[inline]
            fn eq(&self, other: &$type) -> bool {
                *self == other.index()
            }
        }

        impl core::ops::Add<usize> for $type {
            type Output = Self;
            #[inline]
            fn add(self, other: usize) -> Self {
                // use wrapping add so that it's up to the index type whether or
                // not to check -- e.g. if checks are disabled, they're disabled
                // on both debug and release.
                Self::new(self.index().wrapping_add(other))
            }
        }

        impl core::ops::Sub<usize> for $type {
            type Output = Self;
            #[inline]
            fn sub(self, other: usize) -> Self {
                // use wrapping sub so that it's up to the index type whether or
                // not to check -- e.g. if checks are disabled, they're disabled
                // on both debug and release.
                Self::new(self.index().wrapping_sub(other))
            }
        }

        impl core::ops::AddAssign<usize> for $type {
            #[inline]
            fn add_assign(&mut self, other: usize) {
                *self = *self + other
            }
        }

        impl core::ops::SubAssign<usize> for $type {
            #[inline]
            fn sub_assign(&mut self, other: usize) {
                *self = *self - other;
            }
        }

        impl core::ops::Rem<usize> for $type {
            type Output = Self;
            #[inline]
            fn rem(self, other: usize) -> Self {
                Self::new(self.index() % other)
            }
        }

        impl core::ops::Add<$type> for usize {
            type Output = $type;
            #[inline]
            fn add(self, other: $type) -> $type {
                other + self
            }
        }

        impl core::ops::Sub<$type> for usize {
            type Output = $type;
            #[inline]
            fn sub(self, other: $type) -> $type {
                $type::new(self.wrapping_sub(other.index()))
            }
        }

        impl $crate::Idx for $type {
            #[inline]
            fn from_usize(value: usize) -> Self {
                Self::from(value)
            }

            #[inline]
            fn index(self) -> usize {
                usize::from(self)
            }
        }

        impl From<$type> for usize {
            #[inline]
            fn from(v: $type) -> usize {
                v.index()
            }
        }

        impl From<usize> for $type {
            #[inline]
            fn from(value: usize) -> Self {
                $type::from_usize(value)
            }
        }

        $crate::define_index_type! { @__impl_from_rep_unless_usize $type, $raw }
    };
    (@__impl_from_rep_unless_usize $type:ident, usize) => {};
    (@__impl_from_rep_unless_usize $type:ident, $raw:ident) => {
        impl From<$type> for $raw {
            #[inline]
            fn from(v: $type) -> $raw {
                v.raw()
            }
        }

        impl From<$raw> for $type {
            #[inline]
            fn from(value: $raw) -> Self {
                Self::from_raw(value)
            }
        }
    };
}

/// A macro equivalent to the stdlib's `vec![]`, but producing an `IndexVec`.
#[macro_export]
macro_rules! index_vec {
    ($($tokens:tt)*) => {
        $crate::IndexVec::from_vec(vec![$($tokens)*])
    }
}
