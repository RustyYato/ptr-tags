use super::*;

#[repr(transparent)]
pub struct PtrUnion<Tags: PtrList> {
    raw: RawPtrUnion<Tags>,
    _ty: PhantomData<Tags>,
}

impl<Tags: PtrList> Drop for PtrUnion<Tags> {
    fn drop(&mut self) {
        let (ptr, tag) = self.raw.split();
        unsafe { <Tags as PtrList>::drop_at(ptr, tag) }
    }
}

impl<Tags: PtrList> PtrUnion<Tags> {
    pub fn new<P: ErasablePtr, N: Peano>(ptr: P) -> Self
    where
        Tags: Access<P, N>,
    {
        Self {
            raw: RawPtrUnion::from_raw(P::into_raw(ptr), N::VALUE),
            _ty: PhantomData,
        }
    }

    pub fn set<T, N>(&mut self, value: T)
    where
        T: ErasablePtr,
        N: Peano,
        Tags: Access<T, N>,
    {
        *self = Self::new(value);
    }

    pub fn tag(&self) -> usize {
        self.raw.split().1 as usize
    }

    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        RawPtrUnion::ptr_eq(&this.raw, &other.raw)
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

    pub fn try_to_superset<NewTags, Ns>(self) -> Result<PtrUnion<NewTags>, InvalidAlignment>
    where
        Tags: SubsetOf<NewTags, Ns>,
        NewTags: PtrList,
    {
        self.raw.try_to_superset().map(|raw| PtrUnion {
            raw,
            _ty: PhantomData,
        })
    }

    pub fn to_superset<NewTags, Ns>(self) -> PtrUnion<NewTags>
    where
        Tags: SubsetOf<NewTags, Ns>,
        NewTags: PtrList,
    {
        let raw = self.raw.to_superset();
        PtrUnion {
            raw,
            _ty: PhantomData,
        }
    }

    pub fn reorganize<NewTags, Ns>(self) -> PtrUnion<NewTags>
    where
        Tags: SubsetOf<NewTags, Ns, Remaining = Nil>,
        NewTags: PtrList,
    {
        let raw = self.raw.reorganize();
        PtrUnion {
            raw,
            _ty: PhantomData,
        }
    }

    pub fn try_to_subset<NewTags, Ns>(
        self,
    ) -> Result<PtrUnion<NewTags>, PtrUnion<NewTags::Remaining>>
    where
        NewTags: SubsetOf<Tags, Ns>,
    {
        match self.raw.try_to_subset() {
            Ok(raw) => Ok(PtrUnion {
                raw,
                _ty: PhantomData,
            }),
            Err(raw) => Err(PtrUnion {
                raw,
                _ty: PhantomData,
            }),
        }
    }

    pub fn take<T: ErasablePtr, N: Peano>(self) -> Result<T, PtrUnion<Tags::Remaining>>
    where
        Tags: Access<T, N>,
    {
        self.try_to_subset::<TypeList![T], TypeList![N]>()
            .map(PtrUnion::into_inner)
    }
}

impl<T: ErasablePtr> PtrUnion<TypeList![T]> {
    pub fn into_inner(self) -> T {
        unsafe { self.raw.into_inner() }
    }
}

impl PtrUnion<TypeList![]> {
    pub fn unreachable(self) -> ! {
        self.raw.unreachable()
    }
}
