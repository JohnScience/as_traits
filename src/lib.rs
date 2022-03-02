#![feature(generic_const_exprs)]
#![feature(const_fn_trait_bound)]

use core::mem::size_of;
// use just_prim::{Prim, PrimInt};
use just_prim::PrimInt;
// At the time of writing, AlignConstr<T,AlignConstrArchetype> implements `Copy`
// only when `T: Copy` and `AlignConstrArchetype: Copy`. Having `T: Copy` at
// the time of writing is insufficient but will be when zero-length arrays implement `Copy`.
use align_constr::AlignConstr;

// const version of core::cmp::max
#[inline(always)]
pub const fn max(a: usize, b: usize) -> usize {
    if a > b {
        a
    } else {
        b
    }
}

// trait AsPrimitive: Copy + Sized + Prim
// {
//     fn as_<T>(self) -> T
//     where
//         (Self,T): AsPrimitiveTyPair<L=Self,R=T>;
// }

/// Calculates the smallest necessary padding after `T` for reaching the size of "size archetype" `U`
pub const fn padding<T: Copy, U: Copy>() -> usize {
    max(
        size_of::<AlignConstr<T, U>>(),
        size_of::<AlignConstr<U, T>>(),
    ) - size_of::<AlignConstr<T, U>>()
}

#[cfg(test)]
fn abs_diff<T>(a: T, b: T) -> T
where
    T: Copy + core::ops::Sub<Output = T> + core::cmp::Ord,
{
    core::cmp::max(a, b) - core::cmp::min(a, b)
}

#[cfg(target_endian = "little")]
// repr(C) is necessary to enforce field ordering
#[repr(C)]
#[derive(Clone, Copy)]
struct PaddedPrimInt<T, U>
where
    T: PrimInt + Copy,
    U: PrimInt + Copy,
    // In the ideal world, the following must be deduced
    [(); padding::<T, U>()]:,
{
    align_constr_value: AlignConstr<T, U>,
    _padding: [u8; padding::<T, U>()],
}

impl<T, U> PaddedPrimInt<T, U>
where
    T: PrimInt + Copy,
    U: PrimInt + Copy,
    [(); padding::<T, U>()]:,
{
    pub(crate) const fn new(value: T) -> Self {
        Self {
            _padding: [0u8; padding::<T, U>()],
            align_constr_value: AlignConstr::<T, U>::new(value),
        }
    }

    pub(crate) const fn to_value(self) -> T {
        self.align_constr_value.value
    }
}

#[repr(C)]
union ZeroEndianPaddedUnion<L, R>
where
    L: PrimInt + Sized + Copy,
    R: PrimInt + Sized + Copy,
    AlignConstr<L, R>: Copy,
    AlignConstr<R, L>: Copy,
    [(); padding::<L, R>()]:,
    [(); padding::<R, L>()]:,
{
    l: PaddedPrimInt<L, R>,
    r: PaddedPrimInt<R, L>,
}

impl<L, R> ZeroEndianPaddedUnion<L, R>
where
    L: Sized + PrimInt + Copy,
    R: Sized + PrimInt + Copy,
    AlignConstr<L, R>: Copy,
    AlignConstr<R, L>: Copy,
    [(); padding::<L, R>()]:,
    [(); padding::<R, L>()]:,
{
    fn new_l(l: L) -> ZeroEndianPaddedUnion<L, R> {
        ZeroEndianPaddedUnion::<L, R> {
            l: PaddedPrimInt::<L, R>::new(l),
        }
    }
}

#[cfg(test)]
impl<L, R> ZeroEndianPaddedUnion<L, R>
where
    L: Sized + PrimInt + Copy,
    R: Sized + PrimInt + Copy,
    AlignConstr<L, R>: Copy,
    AlignConstr<R, L>: Copy,
    [(); padding::<L, R>()]:,
    [(); padding::<R, L>()]:,
{
    fn addr(&self) -> usize {
        self as *const Self as usize
    }

    fn l_addr(&self) -> usize {
        &unsafe { self.l } as *const PaddedPrimInt<L, R> as usize
    }

    fn r_addr(&self) -> usize {
        &unsafe { self.r } as *const PaddedPrimInt<R, L> as usize
    }
}

pub trait AsPrimitiveTyPair {
    type L: Copy + Sized + PrimInt;
    type R: Copy + Sized + PrimInt;

    fn as_(l: Self::L) -> Self::R
    where
        AlignConstr<<Self as AsPrimitiveTyPair>::L, <Self as AsPrimitiveTyPair>::R>: Copy,
        [(); padding::<Self::L, Self::R>()]:,
        [(); padding::<Self::R, Self::L>()]:,
    {
        let union_ = ZeroEndianPaddedUnion::<Self::L, Self::R>::new_l(l);
        let padded_r = unsafe { union_.r };
        padded_r.to_value()
    }
}

impl<L, R> AsPrimitiveTyPair for (L, R)
where
    L: Copy + Sized + PrimInt,
    R: Copy + Sized + PrimInt,
{
    type L = L;
    type R = R;
}

#[cfg(test)]
mod tests {
    use super::{abs_diff, PaddedPrimInt, ZeroEndianPaddedUnion};
    use crate::AsPrimitiveTyPair;

    #[test]
    fn union_addr_leq_both_l_addr_and_r_addr() {
        let union_ = ZeroEndianPaddedUnion::<u8, u16>::new_l(5);
        let union_addr = union_.addr();
        assert!(union_addr <= union_.l_addr());
        assert!(union_addr <= union_.r_addr());
    }

    #[test]
    #[cfg(target_endian = "little")]
    fn abs_diff_of_union_addr_and_l_addr_eq_0_when_on_le() {
        let union_ = ZeroEndianPaddedUnion::<u8, u16>::new_l(5);
        assert_eq!(abs_diff(union_.addr(), union_.l_addr()), 0);
    }

    #[test]
    #[cfg(target_endian = "little")]
    fn union_addr_eq_l_addr_when_on_le() {
        let union_ = ZeroEndianPaddedUnion::<u8, u16>::new_l(5);
        assert_eq!(union_.addr(), union_.l_addr());
    }

    #[test]
    #[cfg(target_endian = "little")]
    fn abs_diff_of_l_addr_and_r_addr_eq_0_when_on_le() {
        let union_ = ZeroEndianPaddedUnion::<u8, u16>::new_l(5);
        assert_eq!(abs_diff(union_.l_addr(), union_.r_addr()), 0);
    }

    #[test]
    #[cfg(target_endian = "little")]
    fn l_addr_eq_r_addr_when_on_le() {
        let union_ = ZeroEndianPaddedUnion::<u8, u16>::new_l(5);
        assert_eq!(union_.l_addr(), union_.r_addr());
    }

    #[test]
    fn parameterized_padded_prim_int_has_equal_size_regardless_of_type_order() {
        use core::mem::size_of;
        assert_eq!(
            size_of::<PaddedPrimInt<u8, u16>>(),
            size_of::<PaddedPrimInt<u16, u8>>()
        );
    }

    #[test]
    fn parameterized_padded_prim_int_has_equal_alignment_regardless_of_type_order() {
        use core::mem::align_of;
        assert_eq!(
            align_of::<PaddedPrimInt<u8, u16>>(),
            align_of::<PaddedPrimInt<u16, u8>>()
        );
    }

    #[test]
    fn it_works_when_demoting() {
        assert_eq!(<(u16, u8) as AsPrimitiveTyPair>::as_(5u16), 5u8);
    }

    #[test]
    fn it_works_when_promoting() {
        assert_eq!(<(u8, u16) as AsPrimitiveTyPair>::as_(5u8), 5u16);
    }
}
