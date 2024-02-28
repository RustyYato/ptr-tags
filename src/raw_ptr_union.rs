use super::*;

#[repr(transparent)]
pub(crate) struct RawPtrUnion<Tags> {
    ptr: NonNull<u8>,
    _ty: PhantomData<Tags>,
}

unsafe impl<Tags: Send> Send for RawPtrUnion<Tags> {}
unsafe impl<Tags: Sync> Sync for RawPtrUnion<Tags> {}

impl<Tags> Copy for RawPtrUnion<Tags> {}
impl<Tags> Clone for RawPtrUnion<Tags> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Tags: PtrList> RawPtrUnion<Tags> {
    pub(crate) const fn split(&self) -> (NonNull<()>, u8) {
        let addr = addr(self.ptr).get();
        let tag = addr & Tags::MASK;
        // we must use wrapping_sub here so that we preserve the provenance of the pointer
        let ptr = self.ptr.as_ptr().wrapping_sub(tag);
        let tag = tag as u8;

        if tag >= Tags::LEN {
            unsafe { core::hint::unreachable_unchecked() }
        }

        (unsafe { NonNull::new_unchecked(ptr.cast()) }, tag)
    }

    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        this.ptr == other.ptr
    }

    pub fn ptr_hash<S: core::hash::Hasher>(this: &Self, state: &mut S) {
        core::ptr::hash(this.ptr.as_ptr(), state)
    }

    const fn validate_tag(tag: u8) -> usize {
        #[inline(never)]
        const fn validate_tag_failed() -> ! {
            panic!("invalid tag")
        }

        let tag = tag as usize;

        if tag & Tags::MASK != tag || (tag as u8) >= Tags::LEN {
            validate_tag_failed()
        }

        tag
    }

    pub const fn try_from_raw(ptr: NonNull<()>, tag: u8) -> Result<Self, InvalidAlignment> {
        let addr = addr(ptr).get();

        Self::validate_tag(tag);

        if addr & Tags::MASK == 0 {
            Ok(Self::from_raw(ptr, tag))
        } else {
            Err(InvalidAlignment)
        }
    }

    pub const fn from_raw(ptr: NonNull<()>, tag: u8) -> Self {
        let addr = addr(ptr).get();
        let tag = Self::validate_tag(tag);

        assert!(
            addr & Tags::MASK == 0,
            "Invalid pointer alignment for this PtrUnion"
        );

        Self {
            // SAFETY: we checked that we won't overflow right above
            // validate_tag checks that the tag fits in the MASK and
            // the assert above checks that the pointer fits has enough
            // bits remaining to fit the MASK
            // we must use wrapping_add here so that we preserve the provenance of the pointer
            ptr: unsafe { NonNull::new_unchecked(ptr.as_ptr().cast::<u8>().wrapping_add(tag)) },
            _ty: PhantomData,
        }
    }

    const unsafe fn from_raw_unchecked(ptr: NonNull<()>, tag: u8) -> Self {
        let addr = addr(ptr).get();

        debug_assert!(tag < Tags::LEN);
        debug_assert!(
            addr & Tags::MASK == 0,
            "Invalid pointer alignment for this PtrUnion"
        );

        let tag = tag as usize;
        Self {
            // SAFETY: we checked that we won't overflow right above
            // validate_tag checks that the tag fits in the MASK and
            // the assert above checks that the pointer fits has enough
            // bits remaining to fit the MASK
            // we must use wrapping_add here so that we preserve the provenance of the pointer
            ptr: unsafe { NonNull::new_unchecked(ptr.as_ptr().cast::<u8>().wrapping_add(tag)) },
            _ty: PhantomData,
        }
    }

    pub fn try_to_superset<NewTags, Ns>(self) -> Result<RawPtrUnion<NewTags>, InvalidAlignment>
    where
        Tags: SubsetOf<NewTags, Ns>,
        NewTags: PtrList,
    {
        let (ptr, tag) = self.split();

        let tag = unsafe { Tags::map_tag_to_superset(tag) };

        RawPtrUnion::try_from_raw(ptr, tag)
    }

    pub fn to_superset<NewTags, Ns>(self) -> RawPtrUnion<NewTags>
    where
        Tags: SubsetOf<NewTags, Ns>,
        NewTags: PtrList,
    {
        let (ptr, tag) = self.split();

        let tag = unsafe { Tags::map_tag_to_superset(tag) };

        RawPtrUnion::from_raw(ptr, tag)
    }

    pub fn reorganize<NewTags, Ns>(self) -> RawPtrUnion<NewTags>
    where
        Tags: SubsetOf<NewTags, Ns, Remaining = Nil>,
        NewTags: PtrList,
    {
        assert_eq!(NewTags::LEN, Tags::LEN);
        assert_eq!(NewTags::MASK, Tags::MASK);

        let (ptr, tag) = self.split();

        let tag = unsafe { Tags::map_tag_to_superset(tag) };

        // the new tags have exactly the same number of elements as the current tags
        unsafe { RawPtrUnion::from_raw_unchecked(ptr, tag) }
    }

    #[allow(clippy::manual_map)]
    pub fn try_to_subset<NewTags, Ns>(
        self,
    ) -> Result<RawPtrUnion<NewTags>, RawPtrUnion<NewTags::Remaining>>
    where
        NewTags: SubsetOf<Tags, Ns>,
        NewTags: PtrList,
        NewTags::Remaining: PtrList,
    {
        let (ptr, tag) = self.split();

        match unsafe { NewTags::try_map_tag_to_subset(tag, tag) } {
            // the ptr mask will always be smaller in a subset
            Ok(tag) => Ok(unsafe { RawPtrUnion::from_raw_unchecked(ptr, tag) }),
            Err(new_tag) => Err(unsafe { RawPtrUnion::from_raw_unchecked(ptr, new_tag) }),
        }
    }
}

impl<T: ErasablePtr> RawPtrUnion<TypeList![T]> {
    pub unsafe fn into_inner(self) -> T {
        unsafe { T::from_raw(self.ptr.cast()) }
    }
}

impl RawPtrUnion<TypeList![]> {
    pub fn unreachable(self) -> ! {
        unsafe { core::hint::unreachable_unchecked() }
    }
}
