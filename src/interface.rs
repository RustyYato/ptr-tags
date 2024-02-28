use std::cmp::Ordering;

use thin_ptr::Erasable;

use super::*;

pub enum Nil {}
pub enum Cons<T, Ts> {
    Current(T),
    Tail(Ts),
}

pub struct Z;
pub struct S<T>(T);

pub unsafe trait PtrVariants {
    const LEN: usize;
}

pub unsafe trait Peano {
    const VALUE: u8;
}

unsafe impl<T: Peano> Peano for S<T> {
    const VALUE: u8 = T::VALUE + 1;
}
unsafe impl Peano for Z {
    const VALUE: u8 = 0;
}

pub unsafe trait Access<T, N>: PtrList {
    type Remaining: PtrList;

    unsafe fn new(ptr: NonNull<()>) -> Self;
}

unsafe impl<T: ErasablePtr, Ts: PtrList> Access<T, Z> for Cons<T, Ts> {
    type Remaining = Ts;

    unsafe fn new(ptr: NonNull<()>) -> Self {
        Self::Current(T::from_raw(ptr))
    }
}

unsafe impl<T: ErasablePtr, Ts: Access<U, N>, U, N: Peano> Access<U, S<N>> for Cons<T, Ts> {
    type Remaining = Cons<T, Ts::Remaining>;

    unsafe fn new(ptr: NonNull<()>) -> Self {
        Self::Tail(Ts::new(ptr))
    }
}

pub unsafe trait SubsetOf<Ts: PtrList, Ns>: PtrList {
    type Remaining: PtrList;

    unsafe fn map_tag_to_superset(tag: u8) -> u8;

    unsafe fn try_map_tag_to_subset(tag: u8, new_tag: u8) -> Result<u8, u8>;
}

unsafe impl<Ts: PtrList> SubsetOf<Ts, Nil> for Nil {
    type Remaining = Ts;

    unsafe fn map_tag_to_superset(_tag: u8) -> u8 {
        unsafe { core::hint::unreachable_unchecked() }
    }

    unsafe fn try_map_tag_to_subset(_tag: u8, new_tag: u8) -> Result<u8, u8> {
        Err(new_tag)
    }
}

unsafe impl<T: ErasablePtr, Ts: PtrList, Us, N: Peano, Ns> SubsetOf<Us, Cons<N, Ns>> for Cons<T, Ts>
where
    Us: Access<T, N>,
    Ts: SubsetOf<Us::Remaining, Ns>,
{
    type Remaining = Ts::Remaining;

    unsafe fn map_tag_to_superset(tag: u8) -> u8 {
        if let Some(tag) = tag.checked_sub(1) {
            Ts::map_tag_to_superset(tag)
        } else {
            N::VALUE
        }
    }

    unsafe fn try_map_tag_to_subset(tag: u8, new_tag: u8) -> Result<u8, u8> {
        if N::VALUE == tag {
            Ok(0)
        } else {
            Ts::try_map_tag_to_subset(tag, new_tag - u8::from(N::VALUE < new_tag)).map(|x| x + 1)
        }
    }
}

pub unsafe trait PtrList {
    const LEN: u8;
    const MASK: usize = {
        let len = Self::LEN as usize;
        len.next_power_of_two().wrapping_sub(1)
    };

    unsafe fn into_inner(ptr: NonNull<()>, tag: u8) -> Self;

    unsafe fn drop_at(ptr: NonNull<()>, tag: u8);
}

unsafe impl PtrList for Nil {
    const LEN: u8 = 0;

    unsafe fn into_inner(_ptr: NonNull<()>, _tag: u8) -> Self {
        core::hint::unreachable_unchecked()
    }

    unsafe fn drop_at(_ptr: NonNull<()>, _tag: u8) {
        core::hint::unreachable_unchecked()
    }
}

unsafe impl<T: ErasablePtr, Ts: PtrList> PtrList for Cons<T, Ts> {
    const LEN: u8 = Ts::LEN + 1;

    unsafe fn into_inner(ptr: NonNull<()>, tag: u8) -> Self {
        if let Some(tag) = tag.checked_sub(1) {
            Self::Tail(Ts::into_inner(ptr, tag))
        } else {
            Self::Current(T::from_raw(ptr))
        }
    }

    unsafe fn drop_at(ptr: NonNull<()>, tag: u8) {
        if let Some(tag) = tag.checked_sub(1) {
            Ts::drop_at(ptr, tag)
        } else if core::mem::needs_drop::<T>() {
            let _ = T::from_raw(ptr);
        }
    }
}

pub trait MapperOutput {
    type Output;
    const IS_PROPERTY: bool = false;
}

pub trait Mapper<T: ?Sized>: MapperOutput {
    fn call(self, value: &T) -> Self::Output;
}

pub unsafe trait MapHash {
    unsafe fn map_hash<S: core::hash::Hasher>(ptr: NonNull<()>, tag: u8, state: &mut S);
}

unsafe impl MapHash for Nil {
    unsafe fn map_hash<S: core::hash::Hasher>(_ptr: NonNull<()>, _tag: u8, _state: &mut S) {
        unsafe { core::hint::unreachable_unchecked() }
    }
}

unsafe impl<T, Ts> MapHash for Cons<T, Ts>
where
    T: ErasablePtr + core::ops::Deref,
    Ts: MapHash,
    T::Target: Erasable + core::hash::Hash,
{
    unsafe fn map_hash<S: core::hash::Hasher>(ptr: NonNull<()>, tag: u8, state: &mut S) {
        if let Some(tag) = tag.checked_sub(1) {
            Ts::map_hash(ptr, tag, state)
        } else {
            let ptr = <T::Target>::unerase(ptr);
            core::hash::Hash::hash(ptr.as_ref(), state)
        }
    }
}

pub unsafe trait Map<F: MapperOutput> {
    unsafe fn map(ptr: NonNull<()>, tag: u8, f: F) -> F::Output;
}

unsafe impl<F: MapperOutput> Map<F> for Nil {
    unsafe fn map(_ptr: NonNull<()>, _tag: u8, _f: F) -> <F as MapperOutput>::Output {
        unsafe { core::hint::unreachable_unchecked() }
    }
}

unsafe impl<T, Ts, F> Map<F> for Cons<T, Ts>
where
    T: ErasablePtr + core::ops::Deref,
    Ts: Map<F>,
    F: Mapper<T::Target>,
    T::Target: Erasable,
{
    unsafe fn map(ptr: NonNull<()>, tag: u8, f: F) -> <F as MapperOutput>::Output {
        if let Some(tag) = tag.checked_sub(1) {
            Ts::map(ptr, tag, f)
        } else {
            let ptr = <T::Target>::unerase(ptr);
            f.call(ptr.as_ref())
        }
    }
}

pub struct PartialEqAny(pub(crate) NonNull<()>);

impl MapperOutput for PartialEqAny {
    type Output = bool;
}

impl<T: ?Sized + Erasable + PartialEq> Mapper<T> for PartialEqAny {
    fn call(self, value: &T) -> Self::Output {
        let ptr = unsafe { T::unerase(self.0) };
        value == unsafe { ptr.as_ref() }
    }
}

pub struct EqAny;

impl MapperOutput for EqAny {
    type Output = ();
    const IS_PROPERTY: bool = true;
}

impl<T: ?Sized + Eq> Mapper<T> for EqAny {
    fn call(self, _value: &T) -> Self::Output {}
}

pub struct PartialOrdAny(pub(crate) NonNull<()>);

impl MapperOutput for PartialOrdAny {
    type Output = Option<Ordering>;
}

impl<T: ?Sized + Erasable + PartialOrd> Mapper<T> for PartialOrdAny {
    fn call(self, value: &T) -> Self::Output {
        let ptr = unsafe { T::unerase(self.0) };
        value.partial_cmp(unsafe { ptr.as_ref() })
    }
}

pub struct OrdAny(pub(crate) NonNull<()>);

impl MapperOutput for OrdAny {
    type Output = Ordering;
}

impl<T: ?Sized + Erasable + Ord> Mapper<T> for OrdAny {
    fn call(self, value: &T) -> Self::Output {
        let ptr = unsafe { T::unerase(self.0) };
        value.cmp(unsafe { ptr.as_ref() })
    }
}
