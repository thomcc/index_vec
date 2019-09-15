#![allow(clippy::assertions_on_constants)]

use index_vec::{index_vec, IndexVec};

index_vec::define_index_type! {
    pub struct USize16 = usize;
    MAX_INDEX = u16::max_value() as usize;
    DEFAULT = USize16::from_raw_unchecked(usize::max_value());
}

index_vec::define_index_type! {
    pub struct ZeroMaxIgnore = u16;
    MAX_INDEX = 0;
    DISABLE_MAX_INDEX_CHECK = true;
}

index_vec::define_index_type! {
    pub struct ZeroMax = u16;
    MAX_INDEX = 0;
}

index_vec::define_index_type! {
    pub struct IdxSz = usize;
}

index_vec::define_index_type! {
    pub struct Idx32 = u32;
}

index_vec::define_index_type! {
    pub struct Idx16 = u16;
}

index_vec::define_index_type! {
    pub struct Idx8 = u8;
}

index_vec::define_index_type! {
    pub struct SmallCheckedEarly = u8;
    MAX_INDEX = 0x7f;
}

index_vec::define_index_type! {
    pub struct SmallChecked = u8;
}

index_vec::define_index_type! {
    pub struct SmallUnchecked = u8;
    DISABLE_MAX_INDEX_CHECK = true;
}

index_vec::define_index_type! {
    pub struct SmallUncheckedEarly = u8;
    DISABLE_MAX_INDEX_CHECK = true;
    MAX_INDEX = 0x7f;
}

#[test]
fn test_idx_default_max() {
    assert_eq!(Idx32::MAX_INDEX, u32::max_value() as usize);
    assert_eq!(IdxSz::MAX_INDEX, usize::max_value());
    assert_eq!(Idx16::MAX_INDEX, u16::max_value() as usize);
    assert_eq!(Idx8::MAX_INDEX, u8::max_value() as usize);

    assert!(Idx32::CHECKS_MAX_INDEX);
    assert!(IdxSz::CHECKS_MAX_INDEX);
    assert!(Idx16::CHECKS_MAX_INDEX);
    assert!(Idx8::CHECKS_MAX_INDEX);

    assert!(!ZeroMaxIgnore::CHECKS_MAX_INDEX);
    assert_eq!(ZeroMaxIgnore::MAX_INDEX, 0);
}

#[test]
fn test_idx_arith() {
    assert_eq!(Idx32::new(0), 0usize);
    assert_eq!(Idx32::new(0) + 1, 1usize);
    assert_eq!(1 + Idx32::new(0), 1usize);

    assert_eq!(Idx32::new(1) - 1, 0usize);
    assert_eq!(Idx32::new(5) % 4, 1usize);

    let mut m = Idx32::new(5);
    m += 1;
    assert_eq!(m, 6);

    assert!(Idx32::new(5) < Idx32::new(6));
    assert!(Idx32::new(5) < 6usize);

    assert!(Idx32::new(5) < Idx32::new(6));
    assert!(Idx32::new(5) < 6usize);
    assert!(5usize < Idx32::new(6));
}

#[test]
fn test_idx_checks() {
    let v: u32 = Idx32::new(4).raw();
    assert_eq!(v, 4);

    let u: usize = Idx32::new(4).index();
    assert_eq!(u, 4);

    assert_eq!(SmallCheckedEarly::from_raw_unchecked(0xff).raw(), 0xff);

    assert!(SmallChecked::CHECKS_MAX_INDEX);
    assert!(SmallCheckedEarly::CHECKS_MAX_INDEX);

    assert_eq!(SmallChecked::MAX_INDEX, 255);
    assert_eq!(SmallCheckedEarly::MAX_INDEX, 0x7f);

    assert!(!SmallUnchecked::CHECKS_MAX_INDEX);
    assert!(!SmallUncheckedEarly::CHECKS_MAX_INDEX);

    assert_eq!(SmallUnchecked::MAX_INDEX, 255);
    assert_eq!(SmallUncheckedEarly::MAX_INDEX, 0x7f);

    // all shouldn't panic

    let _ = SmallChecked::from_raw(150);
    let _ = SmallChecked::from_usize(150);
    let _ = SmallChecked::from_usize(255);
    let _ = SmallChecked::from_usize(0);

    let _ = SmallCheckedEarly::from_usize(0x7f);
    let _ = SmallCheckedEarly::from_usize(0);

    let _ = SmallUncheckedEarly::from_raw(0xff);
    let _ = SmallUncheckedEarly::from_usize(150);
    let _ = SmallUncheckedEarly::from_usize(300);
    let _ = SmallUnchecked::from_usize(150);
    let _ = SmallUnchecked::from_usize(300);

    let _ = SmallCheckedEarly::from_raw_unchecked(0xff);
    let _ = SmallCheckedEarly::from_usize_unchecked(150);
    let _ = SmallCheckedEarly::from_usize_unchecked(300);
    let _ = SmallChecked::from_usize_unchecked(300);

    assert_eq!(<USize16 as Default>::default().index(), usize::max_value());

    let _ = ZeroMaxIgnore::new((u16::max_value() as usize) + 1);
    let _ = ZeroMaxIgnore::new(0) + 1;
    // let _ = ZeroMaxIgnore::new(0) - 1;
    let _ = ZeroMaxIgnore::new(2);
    let _ = ZeroMaxIgnore::new((u16::max_value() as usize) + 1);
}

#[test]
#[should_panic]
fn test_idx_sc_cf_raw() {
    let _ = SmallCheckedEarly::from_raw(0xff);
}
#[test]
#[should_panic]
fn test_idx_sc_cf_idx0() {
    let _ = SmallCheckedEarly::from_usize(150);
}
#[test]
#[should_panic]
fn test_idx_sc_cf_idx1() {
    let _ = SmallCheckedEarly::from_usize(300);
}
#[test]
#[should_panic]
fn test_idx_sc_cf_idx2() {
    let _ = SmallChecked::from_usize(300);
}
#[test]
#[should_panic]
fn test_idx_sc_of_add() {
    let _ = SmallChecked::from_usize(255) + 1;
}
#[test]
#[should_panic]
fn test_idx_sc_of_addassign() {
    let mut e2 = SmallChecked::from_usize(255);
    e2 += 1;
}
#[test]
#[should_panic]
fn test_idx_sc_of_sub() {
    let _ = SmallChecked::from_usize(0) - 1;
}
#[test]
#[should_panic]
fn test_idx_sc_of_subassign() {
    let mut z2 = SmallChecked::from_usize(0);
    z2 -= 1;
}

#[test]
#[should_panic]
fn test_idx_zm_cf_idx() {
    let _ = ZeroMax::new(2);
}
#[test]
#[should_panic]
fn test_idx_zm_cf_raw() {
    let _ = ZeroMax::from_raw(2);
}

#[test]
#[should_panic]
fn test_idx_zm_of_add0() {
    let _ = ZeroMax::new(0) + 1;
}
#[test]
#[should_panic]
fn test_idx_zm_of_sub0() {
    let _ = ZeroMax::new(0) - 1;
}
#[test]
#[should_panic]
fn test_idx_zm_of_nowrap() {
    let _ = ZeroMax::new((u16::max_value() as usize) + 1);
}

#[test]
#[should_panic]
fn test_idx_sce_adde() {
    let _ = SmallCheckedEarly::from_usize(0x7f) + 1;
}
#[test]
#[should_panic]
fn test_idx_sce_addassign() {
    let mut e3 = SmallCheckedEarly::from_usize(0x7f);
    e3 += 1;
}
#[test]
#[should_panic]
fn test_idx_sce_sub() {
    let _ = SmallCheckedEarly::from_usize(0) - 1;
}
#[test]
#[should_panic]
fn test_idx_sce_subassign() {
    let mut z3 = SmallCheckedEarly::from_usize(0);
    z3 -= 1;
}

#[test]
fn test_vec() {
    let mut strs: IndexVec<Idx32, &'static str> = index_vec!["strs", "bar", "baz"];

    let l = strs.last_idx();
    assert_eq!(strs[l], "baz");

    let new_i = strs.push("quux");
    assert_eq!(strs[new_i], "quux");
}

#[test]
fn test_partial_eq() {
    let i0: IndexVec<Idx32, usize> = index_vec![0];
    let i1: IndexVec<Idx32, usize> = index_vec![1];
    let i123: IndexVec<Idx32, usize> = index_vec![1, 2, 3];

    assert_eq!(i0, i0);
    assert_ne!(i0, i1);
    assert_eq!(i123, vec![1, 2, 3]);
    assert_eq!(i123, &[1, 2, 3]);
    assert_eq!(i123, [1, 2, 3]);
    assert_eq!(i123[..], [1, 2, 3]);
    assert_eq!(i123[..Idx32::new(1)], [1usize]);
    assert_eq!(i123[..Idx32::new(1)], i1.as_slice());
    assert_eq!(i123[..Idx32::new(1)], i1.as_raw_slice());
}

#[test]
fn test_drain() {
    let mut vec: IndexVec<Idx32, usize> = index_vec![1, 2, 3];
    let mut vec2: IndexVec<Idx32, usize> = index_vec![];
    for i in vec.drain(..) {
        vec2.push(i);
    }
    assert!(vec.is_empty());
    assert_eq!(vec2, [1, 2, 3]);

    let mut vec: IndexVec<Idx32, usize> = index_vec![1, 2, 3];
    let mut vec2: IndexVec<Idx32, usize> = index_vec![];
    for i in vec.drain(Idx32::from_raw(1)..) {
        vec2.push(i);
    }
    assert_eq!(vec, [1]);
    assert_eq!(vec2, [2, 3]);

    let mut vec: IndexVec<Idx32, ()> = index_vec![(), (), ()];
    let mut vec2: IndexVec<Idx32, ()> = index_vec![];
    for i in vec.drain(..) {
        vec2.push(i);
    }
    assert_eq!(vec, []);
    assert_eq!(vec2, [(), (), ()]);
}

#[test]
fn test_drain_enumerated() {
    let mut vec: IndexVec<Idx32, usize> = index_vec![1, 2, 3];
    let mut vec2: IndexVec<Idx32, usize> = index_vec![];
    for (i, j) in vec.drain_enumerated(..) {
        assert_eq!(i.index() + 1, j);
        vec2.push(j);
    }
    assert!(vec.is_empty());
    assert_eq!(vec2, [1, 2, 3]);
}
