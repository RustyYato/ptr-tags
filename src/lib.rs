#![allow(clippy::type_complexity)]

use std::{marker::PhantomData, num::NonZeroUsize, ptr::NonNull};

use thin_ptr::ErasablePtr;

mod copy_ptr_union;
mod ptr_union;
mod raw_ptr_union;

use raw_ptr_union::*;

pub use copy_ptr_union::CopyPtrUnion;
pub use ptr_union::PtrUnion;

mod interface;
pub use interface::*;

#[macro_export]
macro_rules! TypeList {
    () => { $crate::Nil };
    ($t:ty $(,)?) => {
        $crate::Cons<$t, $crate::Nil>
    };
    ($t:ty $(, $rest:ty)+ $(,)?) => {
        $crate::Cons<$t, $crate::TypeList![$($rest),*]>
    };
}

#[derive(Clone, Copy)]
pub struct InvalidAlignment;

const fn addr<T>(ptr: NonNull<T>) -> NonZeroUsize {
    unsafe { core::mem::transmute(ptr) }
}

#[test]
fn test_subset_superset() {
    let a = 0u32;

    let x = CopyPtrUnion::<TypeList![&u32, &i32, &mut i32, &u8]>::new(&a);
    assert!(x.is::<&u32, _>());
    let y: CopyPtrUnion<TypeList![&u32, &i32]> = x.try_to_subset().ok().unwrap();
    assert!(y.is::<&u32, _>());
    let y: CopyPtrUnion<TypeList![&i32, &u32]> = x.try_to_subset().ok().unwrap();
    assert!(y.is::<&u32, _>());
    let x: CopyPtrUnion<TypeList![&i32, &u8, &u32, &mut i32]> = x.reorganize();
    assert!(x.is::<&u32, _>());
}

#[test]
fn test_not_included_in_subset() {
    let a = 10u32;

    let x = CopyPtrUnion::<TypeList![&u32, &mut i32, &i32, &u8]>::new(&a);
    assert!(x.is::<&u32, _>());
    let y: Result<CopyPtrUnion<TypeList![&u8, &i32]>, _> = x.try_to_subset();
    let y = y.err().unwrap();
    assert!(y.is::<&u32, _>());
    let y: &u32 = y.take().ok().unwrap();

    assert!(core::ptr::eq(&a, y));
}
