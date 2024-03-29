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

    pub fn ptr_hash<S: core::hash::Hasher>(this: &Self, state: &mut S) {
        RawPtrUnion::ptr_hash(&this.raw, state)
    }

    pub fn map_any<F: MapperOutput>(&self, f: F) -> F::Output
    where
        Tags: Map<F>,
    {
        unsafe { self.raw.map_any(f) }
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

impl<Tags> Eq for PtrUnion<Tags> where Tags: PtrList + Map<EqAny> + Map<PartialEqAny> {}
impl<Tags> PartialEq for PtrUnion<Tags>
where
    Tags: PtrList + Map<PartialEqAny>,
{
    fn eq(&self, other: &Self) -> bool {
        let (ptr, tag) = other.raw.split();
        self.raw.split().1 == tag && self.map_any(PartialEqAny(ptr))
    }
}

impl<Tags> PartialOrd for PtrUnion<Tags>
where
    Tags: PtrList + Map<PartialEqAny> + Map<PartialOrdAny>,
{
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        let (ptr, tag) = other.raw.split();

        match self.raw.split().1.cmp(&tag) {
            o @ (std::cmp::Ordering::Less | std::cmp::Ordering::Greater) => Some(o),
            std::cmp::Ordering::Equal => self.map_any(PartialOrdAny(ptr)),
        }
    }
}

impl<Tags> Ord for PtrUnion<Tags>
where
    Tags: PtrList + Map<PartialEqAny> + Map<PartialOrdAny> + Map<EqAny> + Map<OrdAny>,
{
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        let (ptr, tag) = other.raw.split();

        match self.raw.split().1.cmp(&tag) {
            o @ (std::cmp::Ordering::Less | std::cmp::Ordering::Greater) => o,
            std::cmp::Ordering::Equal => self.map_any(OrdAny(ptr)),
        }
    }
}

impl<Tags> core::hash::Hash for PtrUnion<Tags>
where
    Tags: PtrList + MapHash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        unsafe { self.raw.map_hash(state) }
    }
}
