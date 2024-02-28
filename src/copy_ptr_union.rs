use super::*;

#[repr(transparent)]
pub struct CopyPtrUnion<Tags> {
    raw: RawPtrUnion<Tags>,
}

impl<Tags> Copy for CopyPtrUnion<Tags> {}
impl<Tags> Clone for CopyPtrUnion<Tags> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Tags: PtrList> CopyPtrUnion<Tags> {
    pub fn new<P: ErasablePtr + Copy, N: Peano>(ptr: P) -> Self
    where
        Tags: Access<P, N>,
    {
        Self {
            raw: RawPtrUnion::from_raw(P::into_raw(ptr), N::VALUE),
        }
    }

    pub fn set<T, N>(&mut self, value: T)
    where
        T: ErasablePtr + Copy,
        N: Peano,
        Tags: Access<T, N>,
    {
        *self = Self::new(value);
    }

    pub fn tag(&self) -> usize {
        self.raw.split().1 as usize
    }

    pub fn is<P: ErasablePtr, N: Peano>(&self) -> bool
    where
        Tags: Access<P, N>,
    {
        self.raw.split().1 == N::VALUE
    }

    pub fn try_cast<P: ErasablePtr, N: Peano>(&self) -> Option<P>
    where
        Tags: Access<P, N>,
    {
        if self.is::<P, N>() {
            Some(unsafe { P::from_raw(self.raw.split().0) })
        } else {
            None
        }
    }

    pub fn unpack(&self) -> Tags {
        let (ptr, tag) = self.raw.split();
        unsafe { Tags::into_inner(ptr, tag) }
    }

    pub fn try_to_superset<NewTags, Ns>(self) -> Result<CopyPtrUnion<NewTags>, InvalidAlignment>
    where
        Tags: SubsetOf<NewTags, Ns>,
        NewTags: PtrList,
    {
        self.raw.try_to_superset().map(|raw| CopyPtrUnion { raw })
    }

    pub fn to_superset<NewTags, Ns>(self) -> CopyPtrUnion<NewTags>
    where
        Tags: SubsetOf<NewTags, Ns>,
        NewTags: PtrList,
    {
        let raw = self.raw.to_superset();
        CopyPtrUnion { raw }
    }

    pub fn reorganize<NewTags, Ns>(self) -> CopyPtrUnion<NewTags>
    where
        Tags: SubsetOf<NewTags, Ns, Remaining = Nil>,
        NewTags: PtrList,
    {
        let raw = self.raw.reorganize();
        CopyPtrUnion { raw }
    }

    pub fn try_to_subset<NewTags, Ns>(
        self,
    ) -> Result<CopyPtrUnion<NewTags>, CopyPtrUnion<NewTags::Remaining>>
    where
        NewTags: SubsetOf<Tags, Ns>,
    {
        match self.raw.try_to_subset() {
            Ok(raw) => Ok(CopyPtrUnion { raw }),
            Err(raw) => Err(CopyPtrUnion { raw }),
        }
    }

    pub fn take<T: ErasablePtr, N: Peano>(self) -> Result<T, CopyPtrUnion<Tags::Remaining>>
    where
        Tags: Access<T, N>,
    {
        self.try_to_subset::<TypeList![T], TypeList![N]>()
            .map(CopyPtrUnion::into_inner)
    }
}

impl<T: ErasablePtr> CopyPtrUnion<TypeList![T]> {
    pub fn into_inner(self) -> T {
        unsafe { self.raw.into_inner() }
    }
}

impl CopyPtrUnion<TypeList![]> {
    pub fn unreachable(self) -> ! {
        self.raw.unreachable()
    }
}
