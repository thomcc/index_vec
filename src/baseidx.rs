use core::num::{NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroUsize};

/// Represents a type which can be used as the base index type of a newly defined [`Idx`] type.
///
/// It is implemented for all common unsigned integer types: `u8`, `u16`, `u32`, `u64` and `usize`.
///
/// It also implemented for all common non-zero unsigned integer types: `NonZeroU8`, `NonZeroU16`, `NonZeroU32`, `NonZeroU64` and
/// `NonZeroUsize`.
/// Using non-zero types can be useful for improving performance since it allows the compiler to perform niche optimization in many
/// situations (e.g `size_of::<Option<NonZeroUsize>>() == size_of::<NonZeroUsize>()`).
///
/// For non-zero unsigned integer types, the index value is actually represented internally as `index + 1`, such that index `0` is still
/// a valid index, and the max value is instead the value that is impossible to represent.
///
/// So, for example:
/// - `<NonZeroUsize as BaseIdx>::Converter::from_usize_unchecked(0)` will return `NonZeroUsize(1)`
/// - `<NonZeroUsize as BaseIdx>::Converter::from_usize_unchecked(16)` will return `NonZeroUsize(17)`
/// - `<NonZeroUsize as BaseIdx>::Converter::from_usize(usize::MAX)` will return panic, since after overflow it would theoretically result
///   in `NonZeroUsize(0)`, which is undefined behaviour.
///
/// And, for example:
/// - `<NonZeroUsize as BaseIdx>::Converter::to_usize(NonZeroUsize(1))` will return `Some(NonZeroUsize(0))`
/// - `<NonZeroUsize as BaseIdx>::Converter::to_usize(NonZeroUsize(24))` will return `Some(NonZeroUsize(23))`
///
/// [`Idx`]: crate::Idx
pub trait BaseIdx: Sized {
    /// A converter type, used to convert this value to and from `usize`.
    ///
    /// We could just add conversion functions directly on this trait, (e.g `fn from_usize(v: usize) -> Option<Self>`), but trait
    /// functions can't be called in const contexts, and we want to be able to convert this type to/from `usize` in const contexts.
    ///
    /// Additionally, we can't implement the const conversion functions directly on the type itself, since the type is most likely a
    /// builtin or standard library type (e.g `usize`, `NonZeroUsize`), so we can't implement functions directly on it.
    ///
    /// So what we instead do is implement an additional converter type for every base index type, and then implement the const conversion
    /// functions on that converter type.
    ///
    /// When we then want to convert the base index type to/from `usize`, we call one of the conversion functions of the converter type,
    /// for example: `<NonZeroU16 as BaseIdx>::Converter::from_usize(5_usize)`, which works even in const contexts.
    ///
    /// The functions that are expected to be implemented on the converter type are (`I` represents the base index type):
    /// - `const fn from_usize_unchecked(idx: usize) -> I` - Convert from `usize` to the base index type while skipping as many runtime
    ///   bounds checks as possible. May still panic in some situations where returning the `I` value that corresponds to the provided
    ///   `usize` value would cause undefined behaviour.
    /// - `const fn to_usize(value: I) -> Option<usize>` - Convert from the base index type to `usize`, while performing runtime bounds
    ///   checking. returns `None` if the provided value is too big to fit in a `usize`.
    type Converter;

    /// The maximum value for this base index type.
    const MAX: Self;

    /// The maximum value for this base index type, as a `usize`.
    const MAX_USIZE: usize;
}

macro_rules! impl_base_idx_for_uint {
    {$t: ty} => {
        paste::paste! {
            pub struct [<$t:camel BaseIdxConverter>];
            impl [<$t:camel BaseIdxConverter>] {
                #[inline]
                pub const fn from_usize(idx: usize) -> Option<$t> {
                    // We can't use `try_into` or even the `?` operator here since we are in a const context.
                    // So, we must perform manual bounds checking.
                    if $t::MAX > usize::MAX as $t {
                        // $t is bigger than usize, so we can always just safely convert a usize to $t
                        Some(idx as $t)
                    } else {
                        // $t is smaller than usize, so the conversion may fail
                        if idx <= $t::MAX as usize {
                            Some(idx as $t)
                        } else {
                            None
                        }
                    }
                }

                #[inline]
                pub const fn from_usize_unchecked(idx: usize) -> $t {
                    idx as $t
                }

                #[inline]
                pub const fn to_usize(value: $t) -> Option<usize> {
                    // We can't use `try_into` or even the `?` operator here since we are in a const context.
                    // So, we must perform manual bounds checking.
                    if $t::MAX > usize::MAX as $t {
                        // $t is bigger than usize, we the conversion may fail
                        if value <= usize::MAX as $t {
                            Some(value as usize)
                        } else {
                            None
                        }
                    } else {
                        // $t is smaller than usize, so we can always just safely convert it to a usize
                        Some(value as usize)
                    }
                }
            }
            impl BaseIdx for $t {
                type Converter = [<$t:camel BaseIdxConverter>];
                const MAX: Self = Self::MAX;
                const MAX_USIZE: usize = Self::MAX as usize;
            }
        }
    };
}
impl_base_idx_for_uint! {u8}
impl_base_idx_for_uint! {u16}
impl_base_idx_for_uint! {u32}
impl_base_idx_for_uint! {u64}
impl_base_idx_for_uint! {usize}

/// A manual implementation of the `?` operator for `Option<T>` types, which unlike the `?` operator actually works in const contexts.
macro_rules! opt_try {
    ($x: expr) => {
        (match $x {
            Some(___v) => ___v,
            None => return None
        })
    };
}

macro_rules! impl_base_idx_for_non_zero_uint {
    {$non_zero_uint: ty, $regular_uint: ty} => {
        paste::paste! {
            pub struct [<$non_zero_uint BaseIdxConverter>];
            impl [<$non_zero_uint BaseIdxConverter>] {
                #[inline]
                pub const fn from_usize(idx: usize) -> Option<$non_zero_uint> {
                    Some(
                        // SAFETY: we added 1 and it didn't overflow, so the value is non-zero
                        unsafe {
                            $non_zero_uint::new_unchecked(
                                opt_try!(<$regular_uint as BaseIdx>::Converter::from_usize(
                                    opt_try!(idx.checked_add(1))
                                ))
                            )
                        },
                    )
                }

                #[inline]
                pub const fn from_usize_unchecked(idx: usize) -> $non_zero_uint {
                    // We make a best effort in making this logic "unchecked".
                    //
                    // We don't check the addition and the cast, but we do check that the result is non zero.
                    //
                    // This is very important since creating a non-zero object with a value of zero can cause some really bad undefined
                    // behaviour, and this function is not marked `unsafe`, so it should not cause any such UB under any circumstances.
                    $non_zero_uint::new((idx + 1) as $regular_uint).unwrap()
                }

                #[inline]
                pub const fn to_usize(value: $non_zero_uint) -> Option<usize> {
                    <$regular_uint as BaseIdx>::Converter::to_usize(
                        // SAFETY: the value is non-zero so subtracting 1 can't underflow
                        unsafe { value.get().unchecked_sub(1) }
                    )
                }
            }
            impl BaseIdx for $non_zero_uint {
                type Converter = [<$non_zero_uint BaseIdxConverter>];
                const MAX: Self = Self::MAX;
                const MAX_USIZE: usize = Self::MAX.get() as usize;
            }
        }
    };
}
impl_base_idx_for_non_zero_uint! {NonZeroU8, u8}
impl_base_idx_for_non_zero_uint! {NonZeroU16, u16}
impl_base_idx_for_non_zero_uint! {NonZeroU32, u32}
impl_base_idx_for_non_zero_uint! {NonZeroU64, u64}
impl_base_idx_for_non_zero_uint! {NonZeroUsize, usize}
